use std::time::Duration;

use super::prelude::{error_stage, error_type, io_error_code};
use metrics::{counter, histogram};
use tokio::time::error::Elapsed;
use vector_core::internal_event::InternalEvent;

#[derive(Debug)]
pub struct ExecEventsReceived<'a> {
    pub count: usize,
    pub command: &'a str,
    pub byte_size: usize,
}

impl InternalEvent for ExecEventsReceived<'_> {
    fn emit(self) {
        trace!(
            message = "Events received.",
            count = self.count,
            byte_size = self.byte_size,
            command = %self.command,
        );
        counter!(
            "component_received_events_total", self.count as u64,
            "command" => self.command.to_owned(),
        );
        counter!(
            "component_received_event_bytes_total", self.byte_size as u64,
            "command" => self.command.to_owned(),
        );
        // deprecated
        counter!(
            "events_in_total", self.count as u64,
            "command" => self.command.to_owned(),
        );
    }
}

#[derive(Debug)]
pub struct ExecFailedError<'a> {
    pub command: &'a str,
    pub error: std::io::Error,
}

impl InternalEvent for ExecFailedError<'_> {
    fn emit(self) {
        error!(
            message = "Unable to exec.",
            command = %self.command,
            error = ?self.error,
            error_type = error_type::COMMAND_FAILED,
            error_code = %io_error_code(&self.error),
            stage = error_stage::RECEIVING,
        );
        counter!(
            "component_errors_total", 1,
            "command" => self.command.to_owned(),
            "error_type" => error_type::COMMAND_FAILED,
            "error_code" => io_error_code(&self.error),
            "stage" => error_stage::RECEIVING,
        );
        // deprecated
        counter!(
            "processing_errors_total", 1,
            "command" => self.command.to_owned(),
            "error_type" => error_type::COMMAND_FAILED,
            "stage" => error_stage::RECEIVING,
        );
    }
}

#[derive(Debug)]
pub struct ExecTimeoutError<'a> {
    pub command: &'a str,
    pub elapsed_seconds: u64,
    pub error: Elapsed,
}

impl InternalEvent for ExecTimeoutError<'_> {
    fn emit(self) {
        error!(
            message = "Timeout during exec.",
            command = %self.command,
            elapsed_seconds = %self.elapsed_seconds,
            error = %self.error,
            error_type = error_type::TIMED_OUT,
            stage = error_stage::RECEIVING,
        );
        counter!(
            "component_errors_total", 1,
            "command" => self.command.to_owned(),
            "error_type" => error_type::TIMED_OUT,
            "stage" => error_stage::RECEIVING,
        );
        // deprecated
        counter!(
            "processing_errors_total", 1,
            "command" => self.command.to_owned(),
            "error_type" => error_type::TIMED_OUT,
            "stage" => error_stage::RECEIVING,
        );
    }
}

#[derive(Debug)]
pub struct ExecCommandExecuted<'a> {
    pub command: &'a str,
    pub exit_status: Option<i32>,
    pub exec_duration: Duration,
}

impl ExecCommandExecuted<'_> {
    fn exit_status_string(&self) -> String {
        match self.exit_status {
            Some(exit_status) => exit_status.to_string(),
            None => "unknown".to_string(),
        }
    }
}

impl InternalEvent for ExecCommandExecuted<'_> {
    fn emit(self) {
        let exit_status = self.exit_status_string();
        trace!(
            message = "Executed command.",
            command = %self.command,
            exit_status = %exit_status,
            elapsed_millis = %self.exec_duration.as_millis(),
        );
        counter!(
            "command_executed_total", 1,
            "command" => self.command.to_owned(),
            "exit_status" => exit_status.clone(),
        );

        histogram!(
            "command_execution_duration_seconds", self.exec_duration,
            "command" => self.command.to_owned(),
            "exit_status" => exit_status,
        );
    }
}

pub enum ExecFailedToSignalChildError {
    SignalError(nix::errno::Errno),
    FailedToMarshalPid(std::num::TryFromIntError),
    NoPid,
}

impl ExecFailedToSignalChildError {
    fn to_error_code(&self) -> String {
        use ExecFailedToSignalChildError::*;

        match self {
            SignalError(err) => format!("errno_{}", err),
            FailedToMarshalPid(_) => String::from("failed_to_marshal_pid"),
            NoPid => String::from("no_pid"),
        }
    }
}

impl std::fmt::Display for ExecFailedToSignalChildError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ExecFailedToSignalChildError::*;

        match self {
            SignalError(err) => write!(f, "errno: {}", err),
            FailedToMarshalPid(err) => write!(f, "failed to marshal pid to i32: {}", err),
            NoPid => write!(f, "child had no pid"),
        }
    }
}

pub struct ExecFailedToSignalChild<'a> {
    pub command: &'a tokio::process::Command,
    pub error: ExecFailedToSignalChildError,
}

impl InternalEvent for ExecFailedToSignalChild<'_> {
    fn emit(self) {
        error!(
            message = %format!("Failed to send SIGTERM to child, aborting early: {}", self.error),
            command = ?self.command.as_std(),
            error_code = %self.error.to_error_code(),
            error_type = error_type::COMMAND_FAILED,
            stage = error_stage::RECEIVING,
        );
        counter!(
            "component_errors_total", 1,
            "command" => format!("{:?}", self.command.as_std()),
            "error_code" => self.error.to_error_code(),
            "error_type" => error_type::COMMAND_FAILED,
            "stage" => error_stage::RECEIVING,
        );
        // deprecated
        counter!(
            "processing_errors_total", 1,
            "command_code" => format!("{:?}", self.command.as_std()),
            "error" => self.error.to_error_code(),
            "error_type" => error_type::COMMAND_FAILED,
            "stage" => error_stage::RECEIVING,
        );
    }
}
