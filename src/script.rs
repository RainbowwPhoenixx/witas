use chumsky::{prelude::*, text::Character};

/// Defines how the TAS should start.
#[derive(Debug, Clone)]
pub enum StartType {
    /// TAS should start immediately
    Now,
    /// TAS should start from a new game
    NewGame,
    /// TAS should start from the given save
    Save(String),
}

#[derive(Debug, Clone)]
pub struct ScriptLine {
    pub relative: bool,
    pub tick: u32,
    pub keys: Vec<char>,
    pub mouse: Option<(i32, i32)>,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub version: u64,
    pub start: StartType,
    pub lines: Vec<ScriptLine>,
}

impl Script {
    pub fn get_parser() -> impl Parser<char, Script, Error = Simple<char>> {
        let padding_no_newline = filter(|c: &char| c.is_inline_whitespace()).repeated();

        let comment = just("//")
            .ignore_then(text::newline().not().repeated())
            .padded();

        let version = text::keyword("version")
            .padded()
            .ignore_then(text::int(10).map(|s: String| s.parse().unwrap()))
            .padded();

        let path = filter(|c: &char| !c.is_ascii_control()).repeated();

        let start = text::keyword("start").padded().ignore_then(
            text::keyword("newgame")
                .to(StartType::NewGame)
                .or(text::keyword("now").to(StartType::Now))
                .or(text::keyword("save")
                    .padded()
                    .then(path)
                    .map(|(_, str)| StartType::Save(String::from_iter(str)))),
        );

        let tick = just('+')
            .or_not()
            .then(text::int(10).map(|s: String| s.parse().unwrap()));

        let key = one_of("UuDdLlRrSsPp");

        let signed_int = just('-')
            .or_not()
            .then(text::int(10))
            .map(|(sign, num)| match sign {
                Some(_) => -num.parse::<i32>().unwrap(),
                None => num.parse::<i32>().unwrap(),
            });

        let line = tick
            .then_ignore(just('>'))
            .then(key.repeated())
            .then(
                just('|')
                    .ignore_then(signed_int)
                    .then_ignore(padding_no_newline)
                    .then(signed_int)
                    .or_not(),
            )
            .then_ignore(text::newline())
            .map(|(((is_relative, tick), keys), mouse)| ScriptLine {
                relative: is_relative.is_some(),
                tick,
                keys,
                mouse,
            });

        let lines = line
            .padded_by(comment.repeated())
            .padded()
            .repeated()
            .at_least(1);

        version
            .then(start)
            .then(lines)
            .then_ignore(text::newline().repeated())
            .then_ignore(end())
            .map(|((version, start), lines)| Script {
                version,
                start,
                lines,
            })
    }

    /// Performs additionnal checks on the script.
    pub fn pre_process(&mut self) -> Result<(), String> {
        // Check version
        if self.version != 0 {
            return Err(format!("Invalid version {}", self.version));
        }

        // Set all relative ticks to absolute and check
        // that they are increasing
        let mut tick = self.lines[0].tick;
        for line in &mut self.lines[1..] {
            if line.relative {
                line.relative = false;
                line.tick += tick
            }

            if tick >= line.tick {
                return Err(format!("Expected tick bigger than {tick}."));
            }

            tick = line.tick;
        }

        Ok(())
    }
}
