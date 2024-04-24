use chumsky::prelude::*;

#[derive(Debug, Clone)]
pub struct ScriptLine {
    pub tick: u64,
    pub keys: Vec<char>,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub version: u64,
    pub lines: Vec<ScriptLine>,
}

pub fn parser() -> impl Parser<char, Script, Error = Simple<char>> {
    let version = text::keyword("version")
        .padded()
        .ignore_then(text::int(10).map(|s: String| s.parse().unwrap()))
        .padded();

    let key = filter(|c: &char| c.is_alphabetic());

    let lines = text::int(10)
        .padded()
        .then_ignore(just('>'))
        .then(key.repeated())
        .then_ignore(text::newline())
        .repeated()
        .at_least(1)
        .map(|lines| {
            lines
                .into_iter()
                .map(|(tick, keys)| ScriptLine {
                    tick: tick.parse().unwrap(),
                    keys,
                })
                .collect()
        });

    version
        .then(lines)
        .then_ignore(text::newline().repeated())
        .then_ignore(end())
        .map(|(version, lines)| Script { version, lines })
}
