use crate::auth::AuthUser;
use crate::models::judgehosts::hosts::ActiveModel as JudgehostsActiveModel;
use crate::models::judgehosts::hosts::Entity as JudgehostsEntity;
use crate::models::judgements::ActiveModel as JudgementActiveModel;
use crate::models::judgements::Entity as JudgementsEntity;
use crate::models::problems::Entity as ProblemsEntity;
use crate::models::runs::ActiveModel as RunsActiveModel;
use crate::models::runs::Entity as RunsEntity;
use crate::redis_client::RedisClient;
use axum::{Extension, Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Tasks {
    pub judgement_id: i32,
    pub submission_id: i32,
    pub language_id: String,
    pub problem_id: String,
    pub team_id: String,
    pub contest_time: String,
    pub test_data_count: i32,     // Number of test cases for this problem
    pub completed_runs: Vec<i32>, // List of ordinals that have already been judged
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Runs {
    // pub judgement_id: String,
    pub ordinal: i32, // Ordering of runs in the judgement. Must be different for every run in a judgement. Runs for the same test case must have the same ordinal. Must be between 1 and problem:test_data_count.
    pub judgement_type_id: String,
    pub time: String,
    pub contest_time: String,
    pub run_time: f32,
    pub internal_server_error: bool,
    pub panic_message: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CompileInfo {
    pub error: bool,
    pub start_time: String,         // Absolute time when judgement started.
    pub start_contest_time: String, // Contest relative time when judgement started.
    pub end_time: String,
    pub end_contest_time: String,
    pub max_run_time: Option<f32>,
    pub compile_warning: Option<String>,
    pub compile_error: Option<String>,
    pub judgement_id: i32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Judgements {
    pub id: i32, // ID
    pub submission_id: i32,
    pub judgement_type_id: Option<String>,
    pub simplified_judgement_type_id: Option<String>,
    pub score: f32,
    pub current: Option<bool>,
    pub start_time: String,         // Absolute time when judgement started.
    pub start_contest_time: String, // Contest relative time when judgement started.
    pub end_time: String,
    pub end_contest_time: String,
    pub max_run_time: Option<f32>,
}
pub async fn get_front(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Extension(redis): Extension<RedisClient>,
) -> Result<Json<Tasks>, (StatusCode, Json<ErrorResponse>)> {
    if !auth_user.role.is_judge() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only judgehosts can get queue front.".to_string(),
            }),
        ));
    }

    let mut redis_conn: redis::aio::ConnectionManager = redis.get_connection();

    // Get queue length to determine task availability
    let queue_length: usize = redis_conn.llen("judge_queue").await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Redis error: {}", e),
            }),
        )
    })?;

    if queue_length == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "No tasks in queue".to_string(),
            }),
        ));
    }

    // Count active judgehosts to optimize task distribution
    let active_judgehosts_count = JudgehostsEntity::find()
        .filter(crate::models::judgehosts::hosts::Column::Status.eq("active"))
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to count judgehosts: {}", e),
                }),
            )
        })?
        .len();

    // Calculate advised concurrency per judgehost
    // If we have more judgehosts than tasks, each gets at most 1 task
    // Otherwise, distribute tasks evenly
    // let advised_tasks_per_host = if active_judgehosts_count == 0 {
    //     1 // Default to 1 if no active hosts found yet
    // } else {
    //     let base = queue_length / active_judgehosts_count as usize;
    //     std::cmp::max(1, base) // At least 1 task per host
    // };

    // Check current judgehost's active task count in Redis
    let judgehost_id = &auth_user.username;
    // let active_tasks_key = format!("judgehost:{}:active_tasks", judgehost_id);
    // let current_active_tasks: usize = redis_conn
    //     .get(&active_tasks_key)
    //     .await
    //     .unwrap_or(0);

    // // If this judgehost already has enough tasks, suggest waiting
    // if current_active_tasks >= advised_tasks_per_host {
    //     return Err((
    //         StatusCode::TOO_MANY_REQUESTS,
    //         Json(ErrorResponse {
    //             error: format!(
    //                 "Judgehost has {} active tasks (advised: {}). Active judgehosts: {}, Queue: {}",
    //                 current_active_tasks, advised_tasks_per_host, active_judgehosts_count, queue_length
    //             ),
    //         }),
    //     ));
    // }

    let task_json: Option<String> = redis_conn.lindex("judge_queue", 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Redis error: {}", e),
            }),
        )
    })?;

    let task_str: String = task_json.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "No tasks in queue".to_string(),
        }),
    ))?;

    let task: Tasks = serde_json::from_str(&task_str).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse task: {}", e),
            }),
        )
    })?;

    // Check if judgement has timed out (> 1 minute)
    let judgement = JudgementsEntity::find_by_id(task.judgement_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query judgement: {}", e),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Judgement not found".to_string(),
            }),
        ))?;

    // Parse start time and check if exceeded 1 minute
    let start_time = DateTime::parse_from_rfc3339(&judgement.start_time).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse start time: {}", e),
            }),
        )
    })?;

    let elapsed = Utc::now().signed_duration_since(start_time);
    if elapsed.num_seconds() > 60 {
        // Pop from queue and decrement active tasks
        let _: Option<String> = redis_conn.lpop("judge_queue", None).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Redis error: {}", e),
                }),
            )
        })?;

        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!(
                    "Judge time exceeded 1 minute (elapsed: {} seconds)",
                    elapsed.num_seconds()
                ),
            }),
        ));
    }

    // Update judgehost's last_judge timestamp
    let judgehost = JudgehostsEntity::find_by_id(judgehost_id.clone())
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query judgehost: {}", e),
                }),
            )
        })?;

    if let Some(host) = judgehost {
        let mut host_active: JudgehostsActiveModel = host.into();
        host_active.last_judge = Set(Utc::now().to_rfc3339());
        host_active.status = Set("active".to_string());

        host_active.update(&db).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update judgehost: {}", e),
                }),
            )
        })?;
    }

    // Get problem details to know test_data_count
    let problem = ProblemsEntity::find_by_id(task.problem_id.clone())
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query problem: {}", e),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Problem not found".to_string(),
            }),
        ))?;

    // Get completed runs for this judgement
    let completed_runs = RunsEntity::find()
        .filter(crate::models::runs::Column::JudgementId.eq(task.judgement_id))
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query runs: {}", e),
                }),
            )
        })?;

    // Extract ordinals of completed runs
    let completed_ordinals: Vec<i32> = completed_runs.into_iter().map(|run| run.ordinal).collect();

    // Create enriched task response
    let enriched_task = Tasks {
        judgement_id: task.judgement_id,
        submission_id: task.submission_id,
        language_id: task.language_id,
        problem_id: task.problem_id,
        team_id: task.team_id,
        contest_time: task.contest_time,
        test_data_count: problem.test_data_count,
        completed_runs: completed_ordinals,
    };

    // Increment active task count for this judgehost
    // let _: () = redis_conn.incr(&active_tasks_key, 1).await.map_err(|e| {
    //     (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         Json(ErrorResponse {
    //             error: format!("Redis error: {}", e),
    //         }),
    //     )
    // })?;

    // // Set expiration to 2 minutes (cleanup stale counters)
    // let _: () = redis_conn.expire(&active_tasks_key, 120).await.map_err(|e| {
    //     (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         Json(ErrorResponse {
    //             error: format!("Redis error: {}", e),
    //         }),
    //     )
    // })?;

    Ok(Json(enriched_task))
}

// rewrite

pub async fn handle_judge(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Extension(redis): Extension<RedisClient>,
    Json(payload): Json<Vec<Runs>>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !auth_user.role.is_judge() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only judgehosts can handle queue front.".to_string(),
            }),
        ));
    }
    let mut redis_conn = redis.get_connection();
    let task_json: Option<String> = redis_conn.lindex("judge_queue", 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Redis error: {}", e),
            }),
        )
    })?;

    let task_str: String = task_json.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "No tasks in queue".to_string(),
        }),
    ))?;

    // 4. 将 JSON 字符串反序列化为 Tasks 结构体
    let task: Tasks = serde_json::from_str(&task_str).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse task: {}", e),
            }),
        )
    })?;

    // First, insert all runs
    for verdict in payload.into_iter() {
        if verdict.internal_server_error {
            let _: Option<String> = redis_conn.lpop("judge_queue", None).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Redis error: {}", e),
                    }),
                )
            })?;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to parse task: {}", verdict.panic_message),
                }),
            ));
        }
        let res: RunsActiveModel = RunsActiveModel {
            id: NotSet,
            judgement_id: Set(task.judgement_id),
            ordinal: Set(verdict.ordinal),
            judgement_type_id: Set(verdict.judgement_type_id.clone()),
            time: Set(Utc::now().to_rfc3339()),
            contest_time: Set(verdict.contest_time),
            run_time: Set(verdict.run_time),
        };
        let _inserted = res.insert(&db).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to insert run: {}", e),
                }),
            )
        })?;
    }

    // Get the judgement to check start time
    let judgement = JudgementsEntity::find_by_id(task.judgement_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query judgement: {}", e),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Judgement not found".to_string(),
            }),
        ))?;

    // Check if judge time exceeded 1 minute
    let start_time = DateTime::parse_from_rfc3339(&judgement.start_time).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse start time: {}", e),
            }),
        )
    })?;
    let elapsed = Utc::now().signed_duration_since(start_time);
    if elapsed.num_seconds() > 60 {
        let _: Option<String> = redis_conn.lpop("judge_queue", None).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Redis error: {}", e),
                }),
            )
        })?;
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Judge time exceeded 1 minute".to_string(),
            }),
        ));
    }

    // Get all runs for this judgement
    let all_runs = RunsEntity::find()
        .filter(crate::models::runs::Column::JudgementId.eq(task.judgement_id))
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query runs: {}", e),
                }),
            )
        })?;

    // Get problem's test_data_count to check if all runs completed
    let problem = ProblemsEntity::find_by_id(task.problem_id.clone())
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query problem: {}", e),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Problem not found".to_string(),
            }),
        ))?;

    // Determine final judgement status based on runs
    let mut final_status: Option<String> = None;
    let mut has_tle = false;
    let mut has_mle = false;
    let mut has_rte = false;
    let mut has_wa = false;
    let mut all_ac = true;

    for run in &all_runs {
        match run.judgement_type_id.as_str() {
            "TLE" => {
                has_tle = true;
                all_ac = false;
            }
            "MLE" => {
                has_mle = true;
                all_ac = false;
            }
            "RTE" => {
                has_rte = true;
                all_ac = false;
            }
            "WA" => {
                has_wa = true;
                all_ac = false;
            }
            "AC" => {}
            _ => {
                all_ac = false;
            }
        }
    }

    // Check if all runs completed (number of runs equals test_data_count)
    let all_runs_completed = all_runs.len() as i32 >= problem.test_data_count;

    // Determine final status according to priority rules
    if has_tle {
        // If TLE, immediately set to TLE
        final_status = Some("TLE".to_string());
    } else if all_runs_completed {
        // If all runs completed
        if all_ac {
            // All runs are AC
            final_status = Some("AC".to_string());
        } else {
            // Priority: TLE > MLE > RTE > WA
            if has_mle {
                final_status = Some("MLE".to_string());
            } else if has_rte {
                final_status = Some("RTE".to_string());
            } else if has_wa {
                final_status = Some("WA".to_string());
            }
        }
    }

    // If we have a final status, update judgement and pop from queue
    if let Some(status) = final_status {
        // Calculate max_run_time
        let max_run_time = all_runs
            .iter()
            .map(|r| r.run_time)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        // Update judgement
        let mut judgement_active: JudgementActiveModel = judgement.into();
        judgement_active.judgement_type_id = Set(Some(status.clone()));
        judgement_active.simplified_judgement_type_id = Set(Some(status.clone()));
        judgement_active.end_time = Set(Utc::now().to_rfc3339());
        judgement_active.end_contest_time = Set(task.contest_time.clone());
        judgement_active.max_run_time = Set(Some(max_run_time));

        judgement_active.update(&db).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update judgement: {}", e),
                }),
            )
        })?;

        // Pop from queue
        let _: Option<String> = redis_conn.lpop("judge_queue", None).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Redis error: {}", e),
                }),
            )
        })?;

        // Decrement active task count for this judgehost
        let judgehost_id = &auth_user.username;
        let active_tasks_key = format!("judgehost:{}:active_tasks", judgehost_id);
        let current_count: i32 = redis_conn.get(&active_tasks_key).await.unwrap_or(0);

        if current_count > 0 {
            let _: () = redis_conn.decr(&active_tasks_key, 1).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Redis error: {}", e),
                    }),
                )
            })?;
        }

        let message = StatusResponse {
            message: format!("Judgement completed with status: {}", status),
        };
        Ok(Json(message))
    } else {
        // Not all runs completed yet, just acknowledge
        let message = StatusResponse {
            message: "Runs accepted, waiting for more runs".to_string(),
        };
        Ok(Json(message))
    }
}

pub async fn compile_front(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Extension(redis): Extension<RedisClient>,
    Json(payload): Json<CompileInfo>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !auth_user.role.is_judge() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only judgehosts can compile queue front.".to_string(),
            }),
        ));
    }
    let mut redis_conn = redis.get_connection();
    let task_json: Option<String> = redis_conn.lindex("judge_queue", 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Redis error: {}", e),
            }),
        )
    })?;

    let task_str: String = task_json.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "No tasks in queue".to_string(),
        }),
    ))?;

    let task_front: Tasks = serde_json::from_str(&task_str).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse task: {}", e),
            }),
        )
    })?;
    if task_front.judgement_id != payload.judgement_id {
        return Err((
            StatusCode::NOT_ACCEPTABLE,
            Json(ErrorResponse {
                error: "Not queue front".to_string(),
            }),
        ));
    }
    if payload.error {
        let mut redis_conn = redis.get_connection();
        let fronts: Option<String> = redis_conn.lpop("judge_queue", None).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Redis error: {}", e),
                }),
            )
        })?;
        let fronts_tasks: String = fronts.ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "No tasks in queue".to_string(),
            }),
        ))?;
        let task_front: Tasks = serde_json::from_str(&fronts_tasks).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to parse task: {}", e),
                }),
            )
        })?;

        let judgement = JudgementsEntity::find_by_id(task_front.judgement_id)
            .one(&db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to query judgement: {}", e),
                    }),
                )
            })?
            .ok_or((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Judgement not found".to_string(),
                }),
            ))?;
        let mut judgement_active: JudgementActiveModel = judgement.into();
        judgement_active.judgement_type_id = Set(Some("CE".to_string()));
        judgement_active.simplified_judgement_type_id = Set(Some("CE".to_string()));
        judgement_active.compile_warning = Set(payload.compile_warning);
        judgement_active.compile_error = Set(payload.compile_error);
        judgement_active.end_time = Set(Utc::now().to_rfc3339());
        judgement_active.end_contest_time = Set(payload.end_contest_time);
        judgement_active.max_run_time = Set(Some(0.0));

        let _ = judgement_active.update(&db).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update judgement: {}", e),
                }),
            )
        })?;
        let ret = StatusResponse {
            message: "Compile Error".to_string(),
        };
        return Ok(Json(ret));
    }
    let ret = StatusResponse {
        message: "Done!".to_string(),
    };
    Ok(Json(ret))
}
// pub async fn handle_front_run(
//     auth_user: AuthUser,
//     State(db): State<DatabaseConnection>,
//     Extension(redis): Extension<RedisClient>,
//     Json(payload): Json<Vec<Runs>>,
// ) -> Result<Json<Tasks>, (StatusCode, Json<ErrorResponse>)> {
//     if !auth_user.role.is_judge() {
//         return Err((
//             StatusCode::FORBIDDEN,
//             Json(ErrorResponse {
//                 error: "Only judgehosts can list accounts".to_string(),
//             }),
//         ));
//     }

//     // todo query redis for front of queue
//     let mut redis_conn: redis::aio::ConnectionManager = redis.get_connection();
//     let task_json: Option<String> = redis_conn.lpop("judge_queue", None).await.map_err(|e| {
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             Json(ErrorResponse {
//                 error: format!("Redis error: {}", e),
//             }),
//         )
//     })?;

//     // 3. 处理没有任务的情况
//     let task_str: String = task_json.ok_or((
//         StatusCode::NOT_FOUND,
//         Json(ErrorResponse {
//             error: "No tasks in queue".to_string(),
//         }),
//     ))?;

//     // 4. 将 JSON 字符串反序列化为 Tasks 结构体
//     let task: Tasks = serde_json::from_str(&task_str).map_err(|e| {
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             Json(ErrorResponse {
//                 error: format!("Failed to parse task: {}", e),
//             }),
//         )
//     })?;
//     // todo
//     // let judgements: JudgementActiveModel = JudgementActiveModel {
//     //     id: NotSet,
//     //     submission_id: Set(task.submission_id),
//     //     judgement_type_id: Set(Option::from("PD".to_string())),
//     //     simplified_judgement_type_id: Set(Option::from("PD".to_string())),
//     //     score: Set(0.0),
//     //     current: NotSet,
//     //     start_time: todo!(),
//     //     start_contest_time: todo!(),
//     //     end_time: todo!(),
//     //     end_contest_time: todo!(),
//     //     max_run_time: todo!(),
//     // };

//     // let inserted: crate::models::judgements::Model = judgements.insert(&db).await.map_err(|e| {
//     //     (
//     //         StatusCode::INTERNAL_SERVER_ERROR,
//     //         Json(ErrorResponse {
//     //             error: format!("Failed to insert submission: {}", e),
//     //         }),
//     //     )
//     // })?;

//     for verdict in payload.into_iter() {
//         let res: RunsActiveModel = RunsActiveModel {
//             id: NotSet,
//             judgement_id: Set(task.judgement_id),
//             ordinal: Set(verdict.ordinal),
//             judgement_type_id: Set(verdict.judgement_type_id),
//             time: Set(Utc::now().to_rfc3339()),
//             // todo for contest_time
//             contest_time: Set(verdict.contest_time),
//             run_time: Set(verdict.run_time),
//         };
//         let _inserted = res.insert(&db).await.map_err(|e| {
//             (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 Json(ErrorResponse {
//                     error: format!("Failed to insert submission: {}", e),
//                 }),
//             )
//         })?;
//     }

//     Ok(Json(task))
// }
