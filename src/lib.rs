use anyhow::Result;
use log::debug;
use std::io::BufRead;
use std::str::FromStr;
use thiserror::Error;
use winnow::ascii::{dec_uint, space0, space1};
use winnow::combinator::{alt, opt, separated};
use winnow::prelude::*;
use winnow::stream::Accumulate;
use winnow::Parser;

#[derive(Debug, Error)]
pub enum LineNoError {
    #[error("unable to parse line number/range")]
    UnableToParse,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Range {
    /// Inclusive lowerbound. Unbounded if None
    start: Option<usize>,
    /// Inclusive upperbound. Unbounded if None
    end: Option<usize>,
}

impl Range {
    fn matches(&self, line_num: usize) -> bool {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                if end < start {
                    line_num >= end && line_num <= start
                } else {
                    line_num >= start && line_num <= end
                }
            }
            (Some(start), None) => line_num >= start,
            (None, Some(end)) => line_num <= end,
            (None, None) => true,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Filter {
    Number(usize),
    Range(Range),
}

impl Filter {
    fn matches(&self, line_num: usize) -> bool {
        match self {
            Filter::Number(num) => line_num == *num,
            Filter::Range(range) => range.matches(line_num),
        }
    }
}

fn usize(s: &mut &str) -> PResult<usize> {
    dec_uint.parse_next(s)
}

fn range_separator(s: &mut &str) -> PResult<()> {
    let _ = alt(("..", ":")).parse_next(s)?;
    Ok(())
}

fn range(s: &mut &str) -> PResult<Range> {
    let start = opt(usize).parse_next(s)?;
    let _ = range_separator.parse_next(s)?;
    let end = opt(usize).parse_next(s)?;
    Ok(Range { start, end })
}

fn parse_filter(s: &mut &str) -> PResult<Filter> {
    alt((
        range.map(|r| Filter::Range(r)),
        usize.map(|n| Filter::Number(n)),
    ))
    .parse_next(s)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Filters {
    filters: Vec<Filter>,
}

impl Filters {
    #[cfg(test)]
    fn new(filters: Vec<Filter>) -> Self {
        Self { filters }
    }

    fn join(&mut self, other: &Filters) {
        for filter in &other.filters {
            self.filters.push(filter.clone());
        }
    }

    fn filter(&self, input: impl BufRead) -> Result<Vec<(usize, String)>> {
        let lines = input.lines();

        let num_filters = self.filters.len();
        let mut groups = vec![Vec::new(); num_filters];
        let mut num_matches = 0;

        for (i, line) in lines.enumerate() {
            let line_number = i + 1; // Convert index to line number
            let line = line?;

            for (group_idx, filter) in self.filters.iter().enumerate() {
                debug!("{line_number}, {:?}", filter);
                if filter.matches(line_number) {
                    debug!("match");
                    groups[group_idx].push((line_number, line.clone()));
                    num_matches += 1;
                }
            }
        }

        let mut ret = Vec::with_capacity(num_matches);
        for (i, group) in groups.iter().enumerate() {
            let filter = self.filters.get(i).unwrap();
            match filter {
                Filter::Range(range) => {
                    // The only reason we need to match (instead of just adding all items from the group) is to reverse items when the range is backwards
                    match (range.start, range.end) {
                        (Some(start), Some(end)) => {
                            if start > end {
                                for entry in group.iter().rev() {
                                    ret.push(entry.clone());
                                }
                            } else {
                                for entry in group {
                                    ret.push(entry.clone());
                                }
                            }
                        }
                        _ => {
                            for entry in group {
                                ret.push(entry.clone());
                            }
                        }
                    }
                }
                Filter::Number(_) => {
                    let (line_number, line) = group.first().unwrap();
                    ret.push((*line_number, line.to_string()));
                }
            }
        }
        Ok(ret)
    }
}

impl Accumulate<Filter> for Filters {
    fn initial(capacity: Option<usize>) -> Self {
        let filters = match capacity {
            Some(c) => Vec::with_capacity(c),
            None => Vec::new(),
        };
        Self { filters }
    }

    fn accumulate(&mut self, acc: Filter) {
        self.filters.push(acc);
    }
}

fn comma_space(s: &mut &str) -> PResult<()> {
    let _ = ",".parse_next(s)?;
    let _ = space0.parse_next(s)?;
    Ok(())
}

fn req_space(s: &mut &str) -> PResult<()> {
    let _ = space1.parse_next(s)?;
    Ok(())
}

fn separator(s: &mut &str) -> PResult<()> {
    alt((comma_space, req_space)).parse_next(s)?;
    Ok(())
}

fn filters(s: &mut &str) -> PResult<Filters> {
    separated(1.., parse_filter, separator).parse_next(s)
}

impl FromStr for Filters {
    type Err = LineNoError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        filters.parse(s).map_err(|_| LineNoError::UnableToParse)
    }
}

/// Filter input
pub fn filter(mut filters: Vec<Filters>, input: impl BufRead) -> Result<Vec<(usize, String)>> {
    let Some((filter, others)) = filters.split_first_mut() else {
        let mut output = Vec::new();
        for (i, line) in input.lines().enumerate() {
            let line = line?;
            let num = i + 1; // Convert index to line number
            output.push((num, line));
        }
        return Ok(output);
    };

    for other in others {
        filter.join(other);
    }

    filter.filter(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::*;
    use s_string::s;
    use std::io::Cursor;

    // TODO: test error conditions

    const NUM: usize = 1000;

    /// Generate a large string of numbers separated by newlines
    #[fixture]
    pub fn data() -> Cursor<String> {
        let mut foo = Vec::with_capacity(NUM);
        for i in 1..=NUM {
            foo.push(i.to_string());
        }
        Cursor::new(foo.join("\n"))
    }

    #[rstest]
    fn test_no_filters(data: Cursor<String>) {
        let filters = Filters::new(Vec::new());
        let expected: Vec<String> = vec![];
        let actual: Vec<_> = filters
            .filter(data)
            .unwrap()
            .iter()
            .map(|(_, line)| line.clone())
            .collect();
        assert_eq!(expected, actual);
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(3)]
    #[case(10)]
    #[case(100)]
    #[case(100)]
    #[case(9)]
    #[case(99)]
    #[case(999)]
    fn test_single_line(data: Cursor<String>, #[case] n: usize) {
        let s = n.to_string();
        let filters = Filters::from_str(&s).unwrap();
        let expected = vec![s];
        let actual: Vec<_> = filters
            .filter(data)
            .unwrap()
            .iter()
            .map(|(_, line)| line.clone())
            .collect();
        assert_eq!(expected, actual);
    }

    #[rstest]
    #[case("1:2", vec![s!("1"), s!("2")])]
    #[case("1:3", vec![s!("1"), s!("2"), s!("3")])]
    #[case("998:", vec![s!("998"), s!("999"), s!("1000")])]
    fn test_range(data: Cursor<String>, #[case] input: &str, #[case] expected: Vec<String>) {
        let filters = Filters::from_str(input).unwrap();
        let actual: Vec<_> = filters
            .filter(data)
            .unwrap()
            .iter()
            .map(|(_, line)| line.clone())
            .collect();
        assert_eq!(expected, actual);
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(3)]
    #[case(10)]
    #[case(100)]
    #[case(1000)]
    #[case(9)]
    #[case(99)]
    #[case(999)]
    fn test_parse_number_filters(#[case] input: usize) {
        let actual = Filters::from_str(&input.to_string()).unwrap();
        let expected = Filters::new(vec![Filter::Number(input)]);
        assert_eq!(expected, actual);
    }

    #[rstest]
    // Both ends defined
    #[case("1:2", Filters::new(vec![
        Filter::Range(Range {start: Some(1), end: Some(2)})
    ]))]
    #[case("1..2", Filters::new(vec![
        Filter::Range(Range {start: Some(1), end: Some(2)})
    ]))]
    // No upperbound
    #[case("1:", Filters::new(vec![
        Filter::Range(Range{start: Some(1), end: None})
    ]))]
    #[case("1..", Filters::new(vec![
        Filter::Range(Range{start: Some(1), end: None})
    ]))]
    fn test_parse_range_filters(#[case] input: &str, #[case] expected: Filters) {
        let actual = Filters::from_str(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[rstest]
    /// List of numbers
    #[case("1,2,3", Filters::new(vec![
        Filter::Number(1), Filter::Number(2), Filter::Number(3)
    ]))]
    #[case("1 2 3", Filters::new(vec![
        Filter::Number(1), Filter::Number(2), Filter::Number(3)
    ]))]
    #[case("1, 2, 3", Filters::new(vec![
        Filter::Number(1), Filter::Number(2), Filter::Number(3)
    ]))]
    /// List of ranges
    #[case("1:2,2:3,3:4", Filters::new(vec![
        Filter::Range(Range{start: Some(1), end: Some(2)}),
        Filter::Range(Range{start: Some(2), end: Some(3)}),
        Filter::Range(Range{start: Some(3), end: Some(4)})
    ]))]
    #[case("1:2 2:3 3:4", Filters::new(vec![
        Filter::Range(Range{start: Some(1), end: Some(2)}),
        Filter::Range(Range{start: Some(2), end: Some(3)}),
        Filter::Range(Range{start: Some(3), end: Some(4)})
    ]))]
    #[case("1:2, 2:3, 3:4", Filters::new(vec![
        Filter::Range(Range{start: Some(1), end: Some(2)}),
        Filter::Range(Range{start: Some(2), end: Some(3)}),
        Filter::Range(Range{start: Some(3), end: Some(4)})
    ]))]
    // Lists and numbers
    #[case("1,2,3:4,5:6", Filters::new(vec![
        Filter::Number(1),
        Filter::Number(2),
        Filter::Range(Range{start: Some(3), end: Some(4)}),
        Filter::Range(Range{start: Some(5), end: Some(6)}),
    ]))]
    #[case("1 2 3:4 5:6", Filters::new(vec![
        Filter::Number(1),
        Filter::Number(2),
        Filter::Range(Range{start: Some(3), end: Some(4)}),
        Filter::Range(Range{start: Some(5), end: Some(6)}),
    ]))]
    #[case("1, 2, 3:4, 5:6", Filters::new(vec![
        Filter::Number(1),
        Filter::Number(2),
        Filter::Range(Range{start: Some(3), end: Some(4)}),
        Filter::Range(Range{start: Some(5), end: Some(6)}),
    ]))]
    fn test_parse_complex_filters(#[case] input: &str, #[case] expected: Filters) {
        let actual = Filters::from_str(input).unwrap();
        assert_eq!(expected, actual);
    }
}
