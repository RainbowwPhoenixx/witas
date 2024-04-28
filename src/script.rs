use chumsky::{prelude::*, text::Character};

#[derive(Debug, Clone)]
pub struct ScriptLine {
    pub tick: u32,
    pub keys: Vec<char>,
    pub mouse: Option<(i32, i32)>,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub version: u64,
    pub lines: Vec<ScriptLine>,
}

pub fn parser() -> impl Parser<char, Script, Error = Simple<char>> {
    let padding_no_newline = filter(|c: &char| c.is_inline_whitespace()).repeated();

    let version = text::keyword("version")
        .padded()
        .ignore_then(text::int(10).map(|s: String| s.parse().unwrap()))
        .padded();

    let key = filter(|c: &char| c.is_alphabetic());

    let signed_int = just('-')
        .or_not()
        .then(text::int(10))
        .map(|(sign, num)| match sign {
            Some(_) => -num.parse::<i32>().unwrap(),
            None => num.parse::<i32>().unwrap(),
        });

    let line = text::int(10)
        .padded()
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
        .map(|((tick, keys), mouse)| ScriptLine {
            tick: tick.parse().unwrap(),
            keys,
            mouse,
        });

    let lines = line.repeated().at_least(1);

    version
        .then(lines)
        .then_ignore(text::newline().repeated())
        .then_ignore(end())
        .map(|(version, lines)| Script { version, lines })
}
