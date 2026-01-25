//! Terminal Command Parsing
//!
//! Type-safe command representation for terminal input.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use zos_process::Permissions;

/// Parsed terminal command.
///
/// This enum provides type-safe representation of all terminal commands,
/// avoiding stringly-typed command handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Display help information
    Help,

    /// List running processes
    Ps,

    /// List capabilities
    Caps,

    /// Request spawning a new process
    Spawn { process_type: String },

    /// Request killing a process (not yet implemented)
    Kill { pid: u32 },

    /// Grant a capability to another process
    Grant {
        from_slot: u32,
        to_pid: u32,
        permissions: Permissions,
    },

    /// Revoke (delete) a capability
    Revoke { slot: u32 },

    /// Echo text back to the terminal
    Echo { text: String },

    /// Show system uptime
    Time,

    /// Clear the terminal screen
    Clear,

    /// Exit the terminal
    Exit,

    /// Unrecognized command
    Unknown { cmd: String },
}

/// Error returned when parsing a command fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Command requires an argument that was not provided
    MissingArgument { command: &'static str, argument: &'static str },

    /// Argument could not be parsed as expected type
    InvalidArgument { argument: &'static str, reason: &'static str },
}

impl Command {
    /// Parse a command line into a Command.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let cmd = Command::parse("ps");
    /// assert_eq!(cmd, Ok(Command::Ps));
    ///
    /// let cmd = Command::parse("spawn clock");
    /// assert_eq!(cmd, Ok(Command::Spawn { process_type: "clock".to_string() }));
    /// ```
    pub fn parse(line: &str) -> Result<Self, ParseError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let (cmd, args) = match parts.split_first() {
            Some((c, a)) => (*c, a),
            None => return Ok(Command::Unknown { cmd: String::new() }),
        };

        match cmd {
            "help" | "?" => Ok(Command::Help),
            "ps" => Ok(Command::Ps),
            "caps" => Ok(Command::Caps),

            "spawn" => {
                if args.is_empty() {
                    Err(ParseError::MissingArgument {
                        command: "spawn",
                        argument: "process_type",
                    })
                } else {
                    Ok(Command::Spawn {
                        process_type: args[0].to_string(),
                    })
                }
            }

            "kill" => {
                if args.is_empty() {
                    Err(ParseError::MissingArgument {
                        command: "kill",
                        argument: "pid",
                    })
                } else {
                    args[0]
                        .parse::<u32>()
                        .map(|pid| Command::Kill { pid })
                        .map_err(|_| ParseError::InvalidArgument {
                            argument: "pid",
                            reason: "must be a number",
                        })
                }
            }

            "grant" => {
                if args.len() < 3 {
                    return Err(ParseError::MissingArgument {
                        command: "grant",
                        argument: "from_slot, to_pid, perms",
                    });
                }

                let from_slot = args[0].parse::<u32>().map_err(|_| ParseError::InvalidArgument {
                    argument: "from_slot",
                    reason: "must be a number",
                })?;

                let to_pid = args[1].parse::<u32>().map_err(|_| ParseError::InvalidArgument {
                    argument: "to_pid",
                    reason: "must be a number",
                })?;

                let perms_str = args[2];
                let permissions = Permissions {
                    read: perms_str.contains('r'),
                    write: perms_str.contains('w'),
                    grant: perms_str.contains('g'),
                };

                Ok(Command::Grant {
                    from_slot,
                    to_pid,
                    permissions,
                })
            }

            "revoke" => {
                if args.is_empty() {
                    Err(ParseError::MissingArgument {
                        command: "revoke",
                        argument: "slot",
                    })
                } else {
                    args[0]
                        .parse::<u32>()
                        .map(|slot| Command::Revoke { slot })
                        .map_err(|_| ParseError::InvalidArgument {
                            argument: "slot",
                            reason: "must be a number",
                        })
                }
            }

            "echo" => Ok(Command::Echo {
                text: args.join(" "),
            }),

            "time" | "uptime" => Ok(Command::Time),
            "clear" | "cls" => Ok(Command::Clear),
            "exit" | "quit" => Ok(Command::Exit),

            _ => Ok(Command::Unknown {
                cmd: cmd.to_string(),
            }),
        }
    }

    /// Get a user-friendly usage message for this command.
    pub fn usage(&self) -> &'static str {
        match self {
            Command::Help => "help - Display available commands",
            Command::Ps => "ps - List running processes",
            Command::Caps => "caps - List capabilities",
            Command::Spawn { .. } => "spawn <process_type> - Request process spawn",
            Command::Kill { .. } => "kill <pid> - Request process termination",
            Command::Grant { .. } => "grant <slot> <pid> <perms> - Grant capability (perms: r/w/g)",
            Command::Revoke { .. } => "revoke <slot> - Revoke capability",
            Command::Echo { .. } => "echo <text> - Echo text",
            Command::Time => "time - Show system uptime",
            Command::Clear => "clear - Clear the screen",
            Command::Exit => "exit - Exit the terminal",
            Command::Unknown { .. } => "Unknown command",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_commands() {
        assert_eq!(Command::parse("help"), Ok(Command::Help));
        assert_eq!(Command::parse("?"), Ok(Command::Help));
        assert_eq!(Command::parse("ps"), Ok(Command::Ps));
        assert_eq!(Command::parse("caps"), Ok(Command::Caps));
        assert_eq!(Command::parse("time"), Ok(Command::Time));
        assert_eq!(Command::parse("uptime"), Ok(Command::Time));
        assert_eq!(Command::parse("clear"), Ok(Command::Clear));
        assert_eq!(Command::parse("cls"), Ok(Command::Clear));
        assert_eq!(Command::parse("exit"), Ok(Command::Exit));
        assert_eq!(Command::parse("quit"), Ok(Command::Exit));
    }

    #[test]
    fn test_parse_spawn() {
        assert_eq!(
            Command::parse("spawn clock"),
            Ok(Command::Spawn {
                process_type: "clock".to_string()
            })
        );

        assert_eq!(
            Command::parse("spawn"),
            Err(ParseError::MissingArgument {
                command: "spawn",
                argument: "process_type"
            })
        );
    }

    #[test]
    fn test_parse_kill() {
        assert_eq!(Command::parse("kill 42"), Ok(Command::Kill { pid: 42 }));

        assert_eq!(
            Command::parse("kill"),
            Err(ParseError::MissingArgument {
                command: "kill",
                argument: "pid"
            })
        );

        assert_eq!(
            Command::parse("kill abc"),
            Err(ParseError::InvalidArgument {
                argument: "pid",
                reason: "must be a number"
            })
        );
    }

    #[test]
    fn test_parse_grant() {
        let result = Command::parse("grant 1 42 rw");
        assert_eq!(
            result,
            Ok(Command::Grant {
                from_slot: 1,
                to_pid: 42,
                permissions: Permissions {
                    read: true,
                    write: true,
                    grant: false
                }
            })
        );

        let result = Command::parse("grant 0 1 rwg");
        assert_eq!(
            result,
            Ok(Command::Grant {
                from_slot: 0,
                to_pid: 1,
                permissions: Permissions {
                    read: true,
                    write: true,
                    grant: true
                }
            })
        );
    }

    #[test]
    fn test_parse_revoke() {
        assert_eq!(Command::parse("revoke 5"), Ok(Command::Revoke { slot: 5 }));
    }

    #[test]
    fn test_parse_echo() {
        assert_eq!(
            Command::parse("echo hello world"),
            Ok(Command::Echo {
                text: "hello world".to_string()
            })
        );

        assert_eq!(
            Command::parse("echo"),
            Ok(Command::Echo {
                text: String::new()
            })
        );
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(
            Command::parse("foobar"),
            Ok(Command::Unknown {
                cmd: "foobar".to_string()
            })
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(
            Command::parse(""),
            Ok(Command::Unknown { cmd: String::new() })
        );
        assert_eq!(
            Command::parse("   "),
            Ok(Command::Unknown { cmd: String::new() })
        );
    }
}
