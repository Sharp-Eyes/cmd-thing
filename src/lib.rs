use std::{
    collections::{HashMap, HashSet, VecDeque}, fmt::Write, str::{Chars, FromStr}
};

use anyhow::{Context, Result};

trait NextNonWhitespace {
    fn next_non_whitespace(&mut self) -> Option<char>;
}

impl NextNonWhitespace for Chars<'_> {
    fn next_non_whitespace(&mut self) -> Option<char> {
        loop {
            let c = self.next();
            if c.map_or(true, |c| !c.is_whitespace()) {
                return c;
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum ArgMode {
    Argument,       // Positional argument
    FlagName,       // Multi-character flag name
    ShortFlagName,  // Single-character flag name
    FlagValue,      // Multi-character flag value
    ShortFlagValue, // Single-character flag value
    RestIsRaw,      // Double dash followed by space, pass rest as positional args
    Unset,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub arguments: VecDeque<String>,
    pub switches: HashSet<String>,
    pub flags: HashMap<String, VecDeque<String>>,
}

impl Command {
    pub fn parse(command: String) -> Result<Self> {
        let mut name_buf = String::new();
        let mut value_buf = String::new();

        let mut arguments = VecDeque::new();
        let mut switches = HashSet::new();
        let mut flags = HashMap::new();
        let mut arg_mode = ArgMode::Unset;

        let mut chars = command.chars();
        let mut next_c = chars.next_non_whitespace();

        while next_c.is_some() {
            match next_c.unwrap() {
                // Flag or switch name.
                '-' if arg_mode != ArgMode::RestIsRaw => {
                    next_c = chars.next();

                    // Determine short flag, long flag, or rest-is-raw.
                    arg_mode = match next_c {
                        Some('-') => {
                            next_c = chars.next();
                            if next_c.map_or(false, |c| c.is_whitespace()) {
                                println!("----REST-IS-RAW-----");
                                arg_mode = ArgMode::RestIsRaw;
                                next_c = chars.next_non_whitespace();
                                continue;
                            }
                            ArgMode::FlagName
                        }
                        _ => ArgMode::ShortFlagName,
                    };

                    // Collect flag name/switch name(s).
                    loop {
                        match next_c {
                            Some('\\') => anyhow::bail!("Escapes are not valid in names."),
                            Some('"' | '\'') => anyhow::bail!("Quotes are not valid in names."),
                            Some('-') if arg_mode == ArgMode::ShortFlagName => {
                                anyhow::bail!("Short flags cannot contain hyphens")
                            }
                            Some('-') if name_buf.is_empty() => {
                                anyhow::bail!("Flag names cannot start with a hyphen")
                            }
                            Some(c) if c.is_whitespace() => break,
                            Some(c) => name_buf.write_char(c)?,
                            None if name_buf.is_empty() => anyhow::bail!("EOL before flag name."),
                            None => break,
                        }

                        next_c = chars.next();
                    }
                }
                // Flag value or positional argument.
                _ => {
                    arg_mode = match arg_mode {
                        ArgMode::FlagName => ArgMode::FlagValue,
                        ArgMode::ShortFlagName => ArgMode::ShortFlagValue,
                        ArgMode::Unset => ArgMode::Argument,
                        ArgMode::RestIsRaw => ArgMode::RestIsRaw,
                        _ => unreachable!(),
                    };

                    let mut quote = None;
                    loop {
                        match next_c {
                            Some('\\') => match chars.next() {
                                Some(c) => value_buf.write_char(c)?,
                                None => anyhow::bail!("EOL after escape sequence"),
                            },
                            // Opening quotes.
                            Some('"' | '\'') if quote.is_none() => quote = next_c,
                            // Closing quotes.
                            Some('"' | '\'') if quote == next_c => break,
                            // Whitespace when not in quotes.
                            Some(_) if next_c.unwrap().is_whitespace() && quote.is_none() => break,
                            // Any remaining character, includes "other" quotes when quoted.
                            Some(_) => value_buf.write_char(next_c.unwrap())?,
                            None if quote.is_some() => anyhow::bail!("EOL before closing quote"),
                            None => break,
                        };

                        next_c = chars.next();
                    }
                }
            }

            next_c = chars.next_non_whitespace();

            // At this point we have a name and/or a value, and a lookahead to the next
            // non-whitespace char. From this, we can determine whether the name and/or value
            // should be pushed as an argument/switch/flag.
            match arg_mode {
                ArgMode::RestIsRaw => {
                    debug_assert!(name_buf.is_empty(), "argument with nonempty name_buf");
                    arguments.push_back(value_buf.clone());

                    value_buf.clear();
                }
                ArgMode::Argument => {
                    debug_assert!(name_buf.is_empty(), "argument with nonempty name_buf");
                    arguments.push_back(value_buf.clone());

                    value_buf.clear();
                    arg_mode = ArgMode::Unset;
                }
                // A flag name followed by EOL or a new flag -> switch.
                ArgMode::FlagName if next_c.map_or(true, |c| c == '-') => {
                    debug_assert!(!name_buf.is_empty(), "switch with empty name_buf");
                    debug_assert!(value_buf.is_empty(), "switch with nonempty value_buf");
                    switches.insert(name_buf.clone());

                    name_buf.clear();
                    arg_mode = ArgMode::Unset;
                }
                ArgMode::ShortFlagName if next_c.map_or(true, |c| c == '-') => {
                    debug_assert!(!name_buf.is_empty(), "switch with empty name_buf");
                    debug_assert!(value_buf.is_empty(), "switch with nonempty value_buf");

                    // Short switches can be of the form "-abcd":
                    // "a", "b", "c", "d" become separate switches.
                    switches.extend(name_buf.clone().chars().map(|c| c.to_string()));

                    name_buf.clear();
                    arg_mode = ArgMode::Unset;
                }
                // A flag with both a name and a value -> flag.
                ArgMode::FlagValue => {
                    debug_assert!(!name_buf.is_empty(), "flag with empty name_buf");
                    debug_assert!(!value_buf.is_empty(), "flag with empty value_buf");
                    flags
                        .entry(name_buf.clone())
                        .or_insert_with(|| VecDeque::new())
                        .push_back(value_buf.clone());

                    name_buf.clear();
                    value_buf.clear();
                    arg_mode = ArgMode::Unset;
                }
                ArgMode::ShortFlagValue => {
                    debug_assert!(!name_buf.is_empty(), "flag with empty name_buf");
                    debug_assert!(!value_buf.is_empty(), "flag with empty value_buf");

                    // Short flags can be of the form "-abcd 1":
                    // "d" becomes a flag with value 1, the rest become switches.
                    flags
                        .entry(name_buf.pop().unwrap().to_string())
                        .or_insert_with(|| VecDeque::new())
                        .push_back(value_buf.clone());

                    switches.extend(name_buf.clone().chars().map(|c| c.to_string()));

                    name_buf.clear();
                    value_buf.clear();
                    arg_mode = ArgMode::Unset;
                }
                _ => {}
            };
        }

        Ok(Self {
            arguments,
            switches,
            flags,
        })
    }

    pub fn drain_arguments(&mut self) -> VecDeque<String> {
        let mut out = VecDeque::new();
        out.extend(self.arguments.drain(..));
        out
    }

    pub fn get_next_argument(&mut self) -> Result<String> {
        self.arguments
            .pop_front()
            .context("Missing positional argument")
    }

    pub fn parse_next_argument<T: FromStr>(&mut self) -> Result<T>
    where
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        self.get_next_argument()?
            .parse::<T>()
            .context("Couldn't parse positional argument.")
    }

    pub fn drain_flag(&mut self, flag: &str) -> Option<VecDeque<String>> {
        self.flags.remove(flag)
    }

    pub fn get_next_flag(&mut self, flag: &str) -> Result<String> {
        self.flags
            .get_mut(flag)
            .and_then(|values| values.pop_front())
            .context(format!("Missing value for flag {}", flag))
    }

    pub fn parse_next_flag<T: FromStr>(&mut self, flag: &str) -> Result<T>
    where
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        self.get_next_flag(flag)?
            .parse::<T>()
            .context(format!("Couldn't parse flag '{}'", flag))
    }

    pub fn get_switch(&mut self, switch: &str) -> bool {
        self.switches.remove(switch)
    }
}

pub struct Flag<'a, T: FromStr>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    name: &'a str,
    alias: Option<&'a str>,
    default: Option<T>,
}

impl<'a, T: FromStr> Flag<'a, T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            alias: None,
            default: None,
        }
    }

    pub fn alias(mut self, alias: &'a str) -> Self {
        self.alias = Some(alias);
        self
    }

    pub fn default(mut self, default: T) -> Self {
        self.default = Some(default);
        self
    }

    pub fn parse(self, cmd: &mut Command) -> Result<T> {
        let res = match cmd.get_next_flag(self.name) {
            Ok(v) => Ok(v),
            Err(v) if self.alias.is_none() => return Err(v),
            _ => {
                let alias = self.alias.unwrap();
                cmd.get_next_flag(alias).context(format!(
                    "Missing value for flag '{}' ('{}')",
                    self.name, alias
                ))
            }
        };

        match res {
            Ok(v) => v.parse::<T>().context(format!("Couldn't parse flag.")),
            Err(e) => self.default.map_or(Err(e), |v| Ok(v)),
        }
    }
}
