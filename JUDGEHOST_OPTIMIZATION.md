# 判题主机负载均衡优化文档

## 概述

本系统实现了基于判题主机数量的智能任务分配和负载均衡机制，可以根据活跃的判题主机数量动态调整任务分配策略。

## 核心功能

### 1. 智能任务分配 (`get_front` API)

当判题主机调用 `GET /api/judging/get_front` 时，系统会：

#### API 响应格式

```json
{
  "judgement_id": 123,
  "submission_id": 456,
  "language_id": "cpp",
  "problem_id": "problem-A",
  "team_id": "team-1",
  "contest_time": "2026-06-17T10:30:00Z",
  "test_data_count": 10,
  "completed_runs": [1, 2, 3]
}
```

**字段说明：**
- `judgement_id`: 判题记录 ID
- `submission_id`: 提交记录 ID
- `language_id`: 编程语言
- `problem_id`: 问题 ID
- `team_id`: 队伍 ID
- `contest_time`: 比赛时间
- `test_data_count`: 该问题的测试用例总数（判题主机需要运行的测试用例数量）
- `completed_runs`: 已完成的测试用例编号列表（ordinal）

**判题主机应该判哪些测试用例？**

判题主机应该运行所有测试用例（1 到 `test_data_count`），但可以跳过 `completed_runs` 中已有的测试用例。

示例：
- 如果 `test_data_count = 10` 且 `completed_runs = [1, 2, 3]`
- 判题主机应该运行测试用例 4-10（或者重新运行 1-10 全部）
- 提交时使用 `ordinal` 字段标识每个测试用例（1-10）

#### a) 计算建议的任务数量
```
建议任务数 = 队列长度 / 活跃判题主机数量
最小值 = 1（每个判题主机至少可以获取1个任务）
```

#### b) 检查当前判题主机的负载
系统在 Redis 中维护每个判题主机的活跃任务计数：
```
Key: judgehost:{judgehost_id}:active_tasks
过期时间: 120秒（自动清理过期计数器）
```

#### c) 负载控制
- 如果当前判题主机的活跃任务数 >= 建议任务数，返回 `429 Too Many Requests`
- 错误消息包含当前状态：活跃任务数、建议任务数、活跃判题主机数、队列长度

#### d) 任务分配
如果可以分配任务：
1. 检查判题是否超时（> 1分钟）
2. 更新判题主机的 `last_judge` 时间戳
3. 更新判题主机状态为 `active`
4. 增加该判题主机的活跃任务计数
5. 返回任务信息

### 2. 任务完成处理 (`handle_judge` API)

当判题主机提交判题结果到 `POST /api/judging/handle_judge` 时：

#### a) 判题完成时
- 更新判题记录的最终状态
- 从队列中弹出任务
- **减少该判题主机的活跃任务计数**

#### b) 判题未完成时
- 只插入运行结果
- 不弹出队列
- 不修改活跃任务计数

## 判题主机使用指南

### 工作流程

```
1. 判题主机启动
   ↓
2. 轮询 GET /api/judging/get_front
   ↓
3. 如果返回 200 OK
   → 获得任务，开始判题
   → 系统自动增加活跃任务计数
   ↓
4. 如果返回 429 Too Many Requests
   → 当前负载已满，等待一段时间后重试
   → 建议等待 5-10 秒
   ↓
5. 如果返回 404 Not Found
   → 队列为空，等待一段时间后重试
   → 建议等待 2-5 秒
   ↓
6. 如果返回 500 Internal Server Error
   → 任务超时，任务已被清理
   → 立即重新轮询获取下一个任务
   ↓
7. 执行判题
   ↓
8. 提交结果 POST /api/judging/handle_judge
   → 如果判题完成，系统自动减少活跃任务计数
   ↓
9. 返回步骤 2，继续轮询
```

### 示例代码（伪代码）

```python
import requests
import time

JUDGEHOST_URL = "http://localhost:8000"
POLL_INTERVAL = 2  # 秒
RETRY_INTERVAL = 5  # 秒

def get_task():
    """获取判题任务"""
    response = requests.get(
        f"{JUDGEHOST_URL}/api/judging/get_front",
        headers={"Authorization": f"Bearer {token}"}
    )
    
    if response.status_code == 200:
        return response.json()
    elif response.status_code == 429:
        # 负载已满
        error = response.json()
        print(f"负载已满: {error['error']}")
        time.sleep(RETRY_INTERVAL)
        return None
    elif response.status_code == 404:
        # 队列为空
        time.sleep(POLL_INTERVAL)
        return None
    elif response.status_code == 500:
        # 任务超时，立即重试
        print("任务超时，获取下一个任务")
        return None
    else:
        print(f"未知错误: {response.status_code}")
        time.sleep(RETRY_INTERVAL)
        return None

def submit_runs(runs):
    """提交判题结果"""
    response = requests.post(
        f"{JUDGEHOST_URL}/api/judging/handle_judge",
        headers={"Authorization": f"Bearer {token}"},
        json=runs
    )
    return response.json()

def judge_task(task):
    """执行判题逻辑"""
    runs = []
    
    # 获取测试用例总数
    test_data_count = task['test_data_count']
    completed_runs = set(task['completed_runs'])
    
    print(f"问题有 {test_data_count} 个测试用例")
    print(f"已完成: {completed_runs}")
    
    # 判所有测试用例（或者只判未完成的）
    for ordinal in range(1, test_data_count + 1):
        # 可选：跳过已完成的测试用例
        # if ordinal in completed_runs:
        #     continue
        
        # 运行测试用例
        result = run_test_case(
            submission_id=task['submission_id'],
            problem_id=task['problem_id'],
            test_case_number=ordinal
        )
        
        runs.append({
            "ordinal": ordinal,
            "judgement_type_id": result['verdict'],  # "AC", "WA", "TLE", "MLE", "RTE"
            "time": datetime.utcnow().isoformat() + "Z",
            "contest_time": task['contest_time'],
            "run_time": result['run_time'],  # 运行时间（秒）
            "internal_server_error": False,
            "panic_message": ""
        })
        
        # 如果遇到 TLE，可以立即提交（优化：提前终止）
        if result['verdict'] == 'TLE':
            print("遇到 TLE，提前终止判题")
            break
    
    return runs

# 主循环
while True:
    task = get_task()
    if task:
        print(f"获取到任务: judgement_id={task['judgement_id']}")
        runs = judge_task(task)
        result = submit_runs(runs)
        print(f"提交结果: {result['message']}")
```

## 增量判题说明

系统支持**增量判题**，这意味着：

### 场景 1: 首次判题
- `test_data_count = 10`
- `completed_runs = []`
- 判题主机需要运行所有 10 个测试用例

### 场景 2: 部分完成后重新判题
- `test_data_count = 10`
- `completed_runs = [1, 2, 3]`
- 判题主机可以：
  - **选项 A（推荐）**: 只运行剩余的测试用例 4-10
  - **选项 B**: 重新运行所有测试用例 1-10（会覆盖已有结果）

### 场景 3: TLE 提前终止
- 判题主机在测试用例 5 遇到 TLE
- 可以立即提交结果（只包含测试用例 1-5）
- 系统会立即设置判题状态为 TLE 并完成判题

### 判题策略建议

#### 策略 1: 全量判题（简单）
```python
# 每次都运行所有测试用例
for ordinal in range(1, test_data_count + 1):
    run_test_case(ordinal)
```

**优点**: 实现简单，逻辑清晰
**缺点**: 可能重复运行已完成的测试用例

#### 策略 2: 增量判题（高效）
```python
# 只运行未完成的测试用例
for ordinal in range(1, test_data_count + 1):
    if ordinal not in completed_runs:
        run_test_case(ordinal)
```

**优点**: 节省资源，避免重复工作
**缺点**: 需要处理 completed_runs

#### 策略 3: TLE 提前终止（最优）
```python
for ordinal in range(1, test_data_count + 1):
    if ordinal not in completed_runs:
        result = run_test_case(ordinal)
        if result['verdict'] == 'TLE':
            # 立即提交，不继续运行剩余测试用例
            submit_runs(runs)
            break
```

**优点**: 最快发现 TLE，节省判题时间
**缺点**: 实现稍复杂

## 负载均衡特性

### 1. 动态调整
- 当判题主机数量增加时，每个主机的建议任务数自动减少
- 当判题主机数量减少时，每个主机的建议任务数自动增加

### 2. 自动清理
- Redis 中的活跃任务计数器有 120 秒过期时间
- 防止判题主机崩溃后计数器永久存在

### 3. 超时保护
- 判题超过 1 分钟自动清理
- 在 `get_front` 阶段就检测并清理超时任务

## 状态码说明

| 状态码 | 含义 | 判题主机应该做什么 |
|--------|------|-------------------|
| 200 OK | 成功获取任务 | 开始判题 |
| 404 Not Found | 队列为空 | 等待 2-5 秒后重试 |
| 429 Too Many Requests | 负载已满 | 等待 5-10 秒后重试 |
| 500 Internal Server Error | 任务超时或内部错误 | 立即重试获取下一个任务 |
| 403 Forbidden | 权限不足 | 检查认证信息 |

## 监控指标

系统提供以下信息用于监控：

1. **队列长度**: Redis `judge_queue` 的长度
2. **活跃判题主机数**: 数据库中 `status='active'` 的判题主机数量
3. **每个判题主机的活跃任务数**: Redis `judgehost:{id}:active_tasks`
4. **建议任务数**: 在 429 错误消息中返回

## 优化建议

### 对于判题主机开发者：

1. **实现指数退避**: 连续收到 429 时，增加等待时间
2. **并发处理**: 可以同时处理多个任务（直到达到建议任务数）
3. **健康检查**: 定期更新判题主机状态，保持 `status='active'`
4. **错误处理**: 妥善处理所有状态码，避免死循环

### 对于系统管理员：

1. **监控活跃判题主机数**: 确保有足够的判题主机在线
2. **监控队列长度**: 如果队列持续增长，考虑增加判题主机
3. **检查超时任务**: 如果频繁出现超时，检查判题主机性能或增加超时时间
4. **Redis 内存**: 监控 Redis 内存使用，活跃任务计数器会自动过期

## 数据库要求

判题主机必须在 `judgehosts` 表中注册：

```sql
INSERT INTO judgehosts (id, status, last_judge) 
VALUES ('judgehost-1', 'active', '2026-06-17T00:00:00Z');
```

- `id`: 判题主机唯一标识（必须与认证的 username 一致）
- `status`: 状态（建议使用 'active' / 'inactive'）
- `last_judge`: 最后判题时间（系统自动更新）
