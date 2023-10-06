use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "query.pest"]
struct QueryParser;

impl QueryParser {}

pub fn parse(s: &str) -> Option<QueryNode> {
    let pairs = QueryParser::parse(Rule::query, s).unwrap().next().unwrap();

    fn parse_value(pair: Pair<Rule>) -> QueryNode {
        let mut queries: Vec<QueryNode> = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::not => queries.push(QueryNode::Not(Box::new(QueryNode::And(
                    inner_pair.into_inner().map(parse_value).collect(),
                )))),
                Rule::query => queries.push(parse_value(inner_pair)),
                Rule::query_term => queries.push(QueryNode::Term(
                    inner_pair.into_inner().next().unwrap().as_str().into(),
                )),
                Rule::file => queries.push(QueryNode::File(inner_pair.as_str().into())),
                Rule::lang => queries.push(QueryNode::Lang(inner_pair.as_str().into())),
                Rule::query_text => todo!(),
                Rule::term => queries.push(QueryNode::Term(inner_pair.as_str().into())),
                Rule::regex => {
                    queries.push(QueryNode::Regex(inner_pair.into_inner().as_str().into()))
                }
                Rule::exact => {
                    queries.push(QueryNode::Term(inner_pair.into_inner().as_str().into()))
                }
                Rule::WHITESPACE | Rule::re_char | Rule::re_inner | Rule::char | Rule::inner => {
                    unreachable!()
                }
            };
        }

        if queries.len() == 1 {
            return queries.pop().unwrap();
        }

        QueryNode::And(queries)
    }

    Some(parse_value(pairs))
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryNode {
    Or(Vec<QueryNode>),
    And(Vec<QueryNode>),
    Not(Box<QueryNode>),
    Lang(String),
    File(String),
    Term(String),
    Regex(String),
}

pub struct Document {}

impl QueryNode {
    pub fn is_match(&self, doc: &Document) -> bool {
        match &self {
            QueryNode::Or(_) => todo!(),
            QueryNode::And(_) => todo!(),
            QueryNode::Not(_) => todo!(),
            QueryNode::Lang(_) => todo!(),
            QueryNode::File(_) => todo!(),
            QueryNode::Term(_) => todo!(),
            QueryNode::Regex(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::query::*;

    #[test]
    fn parsing() {
        assert_eq!(parse("Test"), Some(QueryNode::Term("Test".into())));

        assert_eq!(
            parse("Test Test2"),
            Some(QueryNode::And(vec![
                QueryNode::Term("Test".into()),
                QueryNode::Term("Test2".into())
            ]))
        );

        assert_eq!(
            parse("\"Test Me\""),
            Some(QueryNode::Term("Test Me".into()))
        );

        assert_eq!(parse("/re*/"), Some(QueryNode::Regex("re*".into())));

        assert_eq!(
            parse("Test \"Foo Bar\" /or.+/"),
            Some(QueryNode::And(vec![
                QueryNode::Term("Test".into()),
                QueryNode::Term("Foo Bar".into()),
                QueryNode::Regex("or.+".into()),
            ]))
        );
    }
}
