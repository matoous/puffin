use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
    Parser,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "query.pest"]
struct QueryParser;

impl QueryParser {}

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};

        PrattParser::new()
            .op(Op::infix(Rule::and, Left) | Op::infix(Rule::or, Left))
            .op(Op::prefix(Rule::not))
    };
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryNode {
    Or {
        lhs: Box<QueryNode>,
        rhs: Box<QueryNode>,
    },
    And {
        lhs: Box<QueryNode>,
        rhs: Box<QueryNode>,
    },
    Not(Box<QueryNode>),
    Lang(String),
    File(String),
    Term(String),
    Regex(String),
}

impl QueryNode {
    pub fn new(s: &str) -> Self {
        let mut pairs = QueryParser::parse(Rule::query, s).unwrap();
        let query = pairs.next().unwrap().into_inner();

        fn parse_value(primary: Pair<Rule>) -> QueryNode {
            match primary.as_rule() {
                Rule::query => parse_value(primary.into_inner().next().unwrap()),
                Rule::atom => parse_value(primary.into_inner().next().unwrap()),
                Rule::file => QueryNode::File(primary.as_str().into()),
                Rule::lang => QueryNode::Lang(primary.as_str().into()),
                Rule::query_text => todo!(),
                Rule::term => QueryNode::Term(primary.as_str().into()),
                Rule::regex => QueryNode::Regex(primary.into_inner().as_str().into()),
                Rule::exact => QueryNode::Term(primary.into_inner().as_str().into()),
                Rule::expr => parse_expr(primary.into_inner()),
                _ => {
                    unreachable!("{:?} not reachable", primary);
                }
            }
        }

        fn parse_expr(primary: Pairs<Rule>) -> QueryNode {
            PRATT_PARSER
                .map_primary(parse_value)
                .map_infix(|lhs, op, rhs| {
                    println!("Lhs:   {:?}", lhs);
                    println!("Op:    {:?}", op);
                    println!("Rhs:   {:?}", rhs);

                    match op.as_rule() {
                        Rule::and => QueryNode::And {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        Rule::or => QueryNode::Or {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        rule => {
                            unreachable!("Expr::parse expected infix operation, found {:?}", rule)
                        }
                    }
                })
                .map_prefix(|op, rhs| match op.as_rule() {
                    Rule::not => QueryNode::Not(Box::new(rhs)),
                    _ => unreachable!(),
                })
                .parse(primary)
        }

        parse_expr(query)
    }
}

#[cfg(test)]
mod tests {
    use crate::query::*;

    #[test]
    fn parsing() {
        assert_eq!(QueryNode::new("Test"), QueryNode::Term("Test".into()));

        assert_eq!(
            QueryNode::new("Test AND Test2"),
            QueryNode::And {
                lhs: Box::new(QueryNode::Term("Test".into())),
                rhs: Box::new(QueryNode::Term("Test2".into()))
            }
        );

        assert_eq!(
            QueryNode::new("\"Test Me\""),
            QueryNode::Term("Test Me".into())
        );

        assert_eq!(QueryNode::new("/re*/"), QueryNode::Regex("re*".into()));

        assert_eq!(
            QueryNode::new("(Foo AND Bar) OR (Baz AND Buz)"),
            QueryNode::Or {
                lhs: Box::new(QueryNode::And {
                    lhs: Box::new(QueryNode::Term("Foo".into())),
                    rhs: Box::new(QueryNode::Term("Bar".into()))
                }),
                rhs: Box::new(QueryNode::And {
                    lhs: Box::new(QueryNode::Term("Baz".into())),
                    rhs: Box::new(QueryNode::Term("Buz".into()))
                }),
            }
        );
    }
}
