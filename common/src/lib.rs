use bincode::{Decode, Encode};
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use url::Url;
use uuid::Uuid;

pub const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();
// Possible status variant of a task.
#[derive(Clone, Default, PartialEq, Encode, Decode)]
pub enum TaskStatus {
    // the task has been started and is currently progressing.
    #[default]
    Active,
    // the task has been aborted.
    Aborted,
    // the task finished successfully.
    Done,
}

#[derive(Clone, Encode)]
#[cfg_attr(feature = "client", derive(Decode))]
pub struct Task {
    // identifier of the task that will be provided when created.
    // #[serde(skip_deserializing)]
    #[bincode(with_serde)]
    pub id: Uuid,
    // timelapse after which the task will be discarded.
    pub duration: u32,
    // Name of service creating the task. Information given by client.
    pub scope: String,
    // Name of the task to identify it in a human readable way.
    pub name: String,
    // description of the task
    pub description: String,
    // encoded binary data that can be associated with the task.
    pub payload: Vec<u8>,
    // Progress in % updated by client with progress/status write access. R
    pub progress: u8,
    pub status: TaskStatus,
    // Tokens to access the task.
    // The first is to view progress and status.
    // Second one is only to change the status to abort.
    // Third one is to update the progress and status.
    pub tokens: (String, String, String),
    // Url where to send push notifications.
    // #[bitcode(with_serde)]
    #[bincode(with_serde)]
    pub push_address: Vec<Url>,
    // if the tash is finished, a message can be sent to provide a description of the result.
    pub description_result: String,
    // a payload can also be set for the result
    pub payload_result: Vec<u8>,
}
#[derive(Decode)]
#[cfg_attr(feature = "client", derive(Encode))]
pub struct NewTask {
    pub duration: u32,
    pub scope: String,
    pub name: String,
    pub description: String,
    #[bincode(with_serde)]
    pub push_address: Vec<Url>,
    pub payload: Vec<u8>,
}

impl NewTask {
    #[allow(clippy::wrong_self_convention)]
    pub fn to_task(self) -> Task {
        Task {
            id: Uuid::new_v4(),
            duration: self.duration,
            scope: self.scope,
            name: self.name,
            description: self.description,
            payload: self.payload,
            progress: 0,
            status: TaskStatus::Active,
            tokens: (
                thread_rng()
                    .sample_iter(Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect(),
                thread_rng()
                    .sample_iter(Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect(),
                thread_rng()
                    .sample_iter(Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect(),
            ),
            push_address: self.push_address,
            payload_result: Vec::new(),
            description_result: String::new(),
        }
    }
}

impl client_plug_traits::NewTask for NewTask {
    fn duration(&self) -> u32 {
        self.duration
    }
    fn scope(&self) -> &str {
        &self.scope
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn push_address(&self) -> &[Url] {
        &self.push_address
    }
    fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl client_plug_traits::Task for Task {}
