use serde::{Serialize, Deserialize};

/// Access information describing which endpoints and properties are visible to the current client,
/// and what capabilities this client has access to or can perform.
///
/// This endpoint provides information about what is accessible to a specific client in a live contest.
/// Clients are not expected to call this endpoint more than once since the response should not
/// normally change during a contest.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AccessResponse {
    /// An array of capabilities that the current client has.
    /// The array may be empty.
    ///
    /// Examples: "contest_start", "team_submit"
    pub capabilities: Vec<String>,

    /// An array of endpoint objects that are visible to the current client.
    /// The array may be empty.
    pub endpoints: Vec<EndpointInfo>,
}

/// Information about an endpoint that is visible to the current client.
///
/// The set of properties listed must always support referential integrity,
/// i.e. if a property with an ID value referring to some type of object is present,
/// the endpoint representing that type of object (and its ID property) must also be present.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EndpointInfo {
    /// The type of the endpoint, e.g. "problems", "submissions", "teams", etc.
    #[serde(rename = "type")]
    pub r#type: String,

    /// An array of supported properties that the current client has visibility to.
    /// The array must not be empty. If the array would be empty, the endpoint object
    /// should instead not be included in the endpoints array.
    pub properties: Vec<String>,
}

impl AccessResponse {
    /// Creates a new AccessResponse with the given capabilities and endpoints.
    pub fn new(capabilities: Vec<String>, endpoints: Vec<EndpointInfo>) -> Self {
        Self {
            capabilities,
            endpoints,
        }
    }

    /// Checks if the client has a specific capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Finds an endpoint by its type.
    pub fn find_endpoint(&self, endpoint_type: &str) -> Option<&EndpointInfo> {
        self.endpoints.iter().find(|e| e.r#type == endpoint_type)
    }

    /// Checks if a specific endpoint type is accessible.
    pub fn has_endpoint(&self, endpoint_type: &str) -> bool {
        self.find_endpoint(endpoint_type).is_some()
    }

    /// Checks if a specific property is visible for a given endpoint type.
    pub fn has_property(&self, endpoint_type: &str, property: &str) -> bool {
        self.find_endpoint(endpoint_type)
            .map(|e| e.has_property(property))
            .unwrap_or(false)
    }
}

impl EndpointInfo {
    /// Creates a new EndpointInfo.
    ///
    /// # Panics
    /// Panics if properties is empty, as per the CLICS specification.
    pub fn new(r#type: String, properties: Vec<String>) -> Self {
        assert!(!properties.is_empty(), "properties must not be empty");
        Self { r#type, properties }
    }

    /// Checks if this endpoint includes a specific property.
    pub fn has_property(&self, property: &str) -> bool {
        self.properties.iter().any(|p| p == property)
    }

    /// Validates that the properties array is not empty.
    pub fn is_valid(&self) -> bool {
        !self.properties.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_response_serialization() {
        let access = AccessResponse {
            capabilities: vec!["team_submit".to_string()],
            endpoints: vec![
                EndpointInfo {
                    r#type: "contest".to_string(),
                    properties: vec!["id".to_string(), "name".to_string()],
                },
                EndpointInfo {
                    r#type: "problems".to_string(),
                    properties: vec!["id".to_string(), "label".to_string()],
                },
            ],
        };

        let json = serde_json::to_string(&access).unwrap();
        assert!(json.contains(r#""type":"contest"#));
        assert!(json.contains(r#""capabilities":["team_submit"]"#));
    }

    #[test]
    fn test_access_response_deserialization() {
        let json = r#"{
            "capabilities": ["contest_start"],
            "endpoints": [
                {
                    "type": "contest",
                    "properties": ["id", "name"]
                },
                {
                    "type": "problems",
                    "properties": ["id", "label"]
                }
            ]
        }"#;

        let access: AccessResponse = serde_json::from_str(json).unwrap();
        assert_eq!(access.capabilities, vec!["contest_start"]);
        assert_eq!(access.endpoints.len(), 2);
        assert_eq!(access.endpoints[0].r#type, "contest");
    }

    #[test]
    fn test_has_capability() {
        let access = AccessResponse {
            capabilities: vec!["team_submit".to_string(), "contest_start".to_string()],
            endpoints: vec![],
        };

        assert!(access.has_capability("team_submit"));
        assert!(access.has_capability("contest_start"));
        assert!(!access.has_capability("admin"));
    }

    #[test]
    fn test_find_endpoint() {
        let access = AccessResponse {
            capabilities: vec![],
            endpoints: vec![
                EndpointInfo {
                    r#type: "problems".to_string(),
                    properties: vec!["id".to_string()],
                },
            ],
        };

        assert!(access.find_endpoint("problems").is_some());
        assert!(access.find_endpoint("teams").is_none());
    }

    #[test]
    fn test_has_property() {
        let access = AccessResponse {
            capabilities: vec![],
            endpoints: vec![
                EndpointInfo {
                    r#type: "problems".to_string(),
                    properties: vec!["id".to_string(), "label".to_string()],
                },
            ],
        };

        assert!(access.has_property("problems", "id"));
        assert!(access.has_property("problems", "label"));
        assert!(!access.has_property("problems", "name"));
        assert!(!access.has_property("teams", "id"));
    }

    #[test]
    #[should_panic(expected = "properties must not be empty")]
    fn test_endpoint_info_empty_properties() {
        EndpointInfo::new("problems".to_string(), vec![]);
    }

    #[test]
    fn test_endpoint_info_validation() {
        let valid = EndpointInfo {
            r#type: "problems".to_string(),
            properties: vec!["id".to_string()],
        };
        assert!(valid.is_valid());

        let invalid = EndpointInfo {
            r#type: "problems".to_string(),
            properties: vec![],
        };
        assert!(!invalid.is_valid());
    }
}