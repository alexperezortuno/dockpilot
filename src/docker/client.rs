use bollard::Docker;
use std::fmt;
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    Connected,
    Disconnected(String),
}

impl EngineStatus {
    pub fn message(&self) -> String {
        match self {
            Self::Connected => "connected".to_string(),
            Self::Disconnected(reason) => format!("disconnected: {}", reason),
        }
    }
}

impl fmt::Display for EngineStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

pub struct EngineConnection {
    client: Option<Docker>,
    status: EngineStatus,
}

impl EngineConnection {
    pub async fn connect() -> Self {
        let client = match Docker::connect_with_local_defaults() {
            Ok(client) => client,
            Err(error) => {
                return Self {
                    client: None,
                    status: EngineStatus::Disconnected(error.to_string()),
                };
            }
        };

        let status = match timeout(Duration::from_secs(2), client.ping()).await {
            Ok(Ok(_)) => EngineStatus::Connected,
            Ok(Err(error)) => EngineStatus::Disconnected(error.to_string()),
            Err(_) => EngineStatus::Disconnected("connection timed out".to_string()),
        };
        let client = if status == EngineStatus::Connected {
            Some(client)
        } else {
            None
        };

        Self { client, status }
    }

    pub fn status(&self) -> &EngineStatus {
        &self.status
    }

    pub fn status_message(&self) -> String {
        self.status.message()
    }

    pub fn into_client(self) -> Option<Docker> {
        self.client
    }
}

#[cfg(test)]
mod tests {
    use super::EngineStatus;

    #[test]
    fn status_messages_are_actionable() {
        assert_eq!(EngineStatus::Connected.message(), "connected");
        assert_eq!(
            EngineStatus::Disconnected("daemon unavailable".to_string()).message(),
            "disconnected: daemon unavailable"
        );
    }
}
