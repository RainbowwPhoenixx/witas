use chumsky::{prelude::*, text::Character};

use crate::witness::witness_types::{Vec2, Vec3};

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
pub enum Tool {
    /// Set the position of the player
    SetPos { pos: Vec3, ang: Vec2 },
}

#[derive(Debug, Clone)]
pub struct ScriptLine {
    pub relative: bool,
    pub tick: u32,
    pub keys: Vec<char>,
    pub mouse: Option<(i32, i32)>,
    pub tools: Option<Tool>,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub version: u64,
    pub start: StartType,
    pub lines: Vec<ScriptLine>,
}

impl Script {
    fn get_parser() -> impl Parser<char, Self, Error = Simple<char>> {
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
            .chain::<char, _, _>(text::int(10))
            .collect::<String>()
            .map(|num| num.parse::<i32>().unwrap());

        let float = just('-')
            .or_not()
            .chain::<char, _, _>(text::int(10))
            .chain::<char, _, _>(just('.'))
            .chain::<char, _, _>(text::digits(10).or_not())
            .collect::<String>()
            .map(|num| num.parse::<f32>().unwrap());

        let coords = signed_int.then_ignore(padding_no_newline).then(signed_int);

        let mouse_move_part = just('|')
            .ignore_then(padding_no_newline)
            .ignore_then(coords.or_not())
            .or_not()
            .map(|c| c.flatten());

        let setpos_tool = just("setpos")
            .ignore_then(padding_no_newline)
            .ignore_then(float)
            .then_ignore(padding_no_newline)
            .chain(float)
            .then_ignore(padding_no_newline)
            .chain(float)
            .then_ignore(padding_no_newline)
            .chain(float)
            .then_ignore(padding_no_newline)
            .chain(float)
            .map(|numbers| Tool::SetPos {
                pos: Vec3 {
                    x: numbers[0],
                    y: numbers[1],
                    z: numbers[2],
                },
                ang: Vec2 {
                    x: numbers[3],
                    y: numbers[4],
                },
            });

        let tools_part = just('|')
            .ignore_then(padding_no_newline)
            .ignore_then(setpos_tool.or_not())
            .or_not()
            .map(|c| c.flatten());

        let line = tick
            .then_ignore(just('>'))
            .then(key.repeated())
            .then(mouse_move_part)
            .then(tools_part)
            .then_ignore(comment.or_not())
            .map(|((((is_relative, tick), keys), mouse), tools)| ScriptLine {
                relative: is_relative.is_some(),
                tick,
                keys,
                mouse,
                tools,
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
    fn pre_process(&mut self) -> Result<(), String> {
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

    pub fn try_from(src: String) -> Result<Self, Vec<String>> {
        match Self::get_parser().parse(src.clone()) {
            Err(parse_errs) => Err(parse_errs
                .iter()
                .map(|e| {
                    let line = src[..e.span().end].match_indices("\n").count() + 1;
                    format!("line {line}: {e}")
                })
                .collect()),
            Ok(mut script) => match script.pre_process() {
                Ok(_) => Ok(script),
                Err(err) => Err(vec![err]),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::script::Script;
    use chumsky::Parser;

    #[test]
    fn test_parser() {
        let script = "
        version 0
        start now
        
        1>|0 0
        2>|
        3>|0 0 
        4>|0 0 // test
        5>|0 0//test
        ";

        let res = Script::get_parser().parse(script);
        assert!(res.is_ok())
    }
}
