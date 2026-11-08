use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StopHookError {
    #[error("gate '{gate}' did not run")]
    GateDidNotRun { gate: String },
    #[error("gate '{gate}' failed with exit code {exit_code}")]
    GateFailed { gate: String, exit_code: i32 },
}

/// Verifies that all required L1–L4 gates returned exit code 0.
/// The model cannot declare a task completed unless this returns Ok(()).
pub fn verify_stop_hook<'a, I>(gates: I) -> Result<(), StopHookError>
where
    I: IntoIterator<Item = (&'a str, Option<i32>)>,
{
    for (name, exit_opt) in gates {
        match exit_opt {
            None => {
                return Err(StopHookError::GateDidNotRun {
                    gate: name.to_string(),
                });
            }
            Some(code) if code != 0 => {
                return Err(StopHookError::GateFailed {
                    gate: name.to_string(),
                    exit_code: code,
                });
            }
            Some(_) => {}
        }
    }
    Ok(())
}
