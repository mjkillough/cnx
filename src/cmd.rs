use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::process::Command;
use std::str::FromStr;

use anyhow::{Context, Result};

fn command_output_inner<C, A>(command: C, args: &[A]) -> Result<String>
where
    C: AsRef<OsStr> + Debug,
    A: AsRef<OsStr> + Debug,
{
    let output = Command::new(command.as_ref()).args(args).output()?;
    let string = String::from_utf8(output.stdout)?;
    Ok(string)
}

pub fn command_output<C, A>(command: C, args: &[A]) -> Result<String>
where
    C: AsRef<OsStr> + Debug,
    A: AsRef<OsStr> + Debug,
{
    let command = command.as_ref();
    let string = command_output_inner(command, args).with_context(|| {
        format!(
            "Running command `{command:?}` `{args:?}`",
            command = command,
            args = args
        )
    })?;
    Ok(string)
}

fn from_str<S, R>(str: S) -> Result<R>
where
    S: AsRef<str>,
    R: FromStr,
    <R as FromStr>::Err: Error + Send + Sync + 'static,
{
    let value = R::from_str(str.as_ref())?;
    Ok(value)
}

pub fn from_command_output<C, A, R>(command: C, args: &[A]) -> Result<R>
where
    C: AsRef<OsStr> + Debug,
    A: AsRef<OsStr> + Debug,
    R: FromStr,
    <R as FromStr>::Err: Error + Send + Sync + 'static,
{
    let command = command.as_ref();
    let string = command_output(command, args)?;
    let value = from_str(string.trim()).with_context(|| {
        format!(
            "Parsing command `{command:?}` `{args:?}`: `{string}`",
            command = command,
            args = args,
            string = string,
        )
    })?;
    Ok(value)
}

