use crate::eval::evaluator::ConditionResult;
use crate::ServedValue;

#[derive(Default)]
pub struct EvalLogBuilder {
    content: String,
    indent: usize,
}

impl EvalLogBuilder {
    const NEW_LINE_CHAR: char = '\n';
    const INDENT_SEQ: &'static str = "  ";

    pub fn reset_indent(&mut self) -> &mut Self {
        self.indent = 0;
        self
    }

    pub fn inc_indent(&mut self) -> &mut Self {
        self.indent += 1;
        self
    }

    pub fn dec_indent(&mut self) -> &mut Self {
        if self.indent == 0 {
            return self;
        }
        self.indent -= 1;
        self
    }

    pub fn new_ln(&mut self, message: Option<&str>) -> &mut Self {
        self.content.push(Self::NEW_LINE_CHAR);
        self.content
            .push_str(Self::INDENT_SEQ.repeat(self.indent).as_str());
        if let Some(msg) = message {
            self.content.push_str(msg);
        }
        self
    }

    pub fn append(&mut self, val: &str) -> &mut Self {
        self.content.push_str(val);
        self
    }

    pub fn append_then_clause(
        &mut self,
        new_line: bool,
        result: &ConditionResult,
        rule_srv_value: &Option<ServedValue>,
    ) -> &mut Self {
        let builder = self.inc_indent();
        if new_line {
            builder.new_ln(None);
        } else {
            builder.append(" ");
        }
        builder.append("THEN");
        if let Some(sv) = rule_srv_value.as_ref() {
            builder.append(format!(" '{}'", sv.value).as_str());
        } else {
            builder.append(" % options");
        }
        builder.append(" => ");
        match result {
            ConditionResult::Success(matched) => {
                if *matched {
                    builder.append("MATCH, applying rule");
                } else {
                    builder.append("no match");
                }
            }
            _ => {
                builder.append(format!("{result}").as_str());
            }
        }
        builder.dec_indent();
        builder
    }

    pub fn content(&self) -> &str {
        self.content.as_str()
    }
}
