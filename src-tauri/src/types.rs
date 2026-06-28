#[derive(serde::Serialize)]
pub struct DetectedTools {
    pub seven_zip: Option<String>,
    pub bandizip: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct LogPayload {
    pub task_id: String,
    pub message: String,
    pub status: String, // "running", "success", "error"
    pub progress: f32,
}

#[derive(serde::Serialize)]
pub struct ExtractResult {
    pub success: bool,
    #[serde(rename = "errorType")]
    pub error_type: String, // "None" | "PasswordRequired" | "Other"
    pub message: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ProgressPayload {
    pub task_id: String,
    pub progress: f32,
}
