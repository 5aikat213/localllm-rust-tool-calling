use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;
use log::{info, error};

#[derive(Error, Debug)]
pub enum PythonInvokerError {
    #[error("Failed to execute Python script: {0}")]
    CommandError(String),
    #[error("Script execution failed: {0}")]
    ScriptError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PythonScriptResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

pub struct PythonInvoker;

impl PythonInvoker {
    pub fn new() -> Self {
        Self
    }

    pub fn run_script(&self, script: &str, args: &[&str]) -> Result<PythonScriptResult, PythonInvokerError> {
        info!("Executing Python script with args: {:?}", args);

        let output = Command::new("python3")
            .arg("-c")
            .arg(script)
            .args(args)
            .output()
            .map_err(|e| PythonInvokerError::CommandError(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();

        if output.status.success() {
            info!("Python script executed successfully");
            Ok(PythonScriptResult {
                stdout,
                stderr,
                exit_code,
            })
        } else {
            error!("Python script execution failed with exit code: {:?}", exit_code);
            Err(PythonInvokerError::ScriptError(format!(
                "Exit Code: {:?}\nStdout: {}\nStderr: {}",
                exit_code, stdout, stderr
            )))
        }
    }
} 