use crate::eval::evaluator::ConditionResult;
use crate::TargetingRule;

#[derive(Default)]
pub struct EvalLogBuilder {
    content: String,
    indent: usize,
}

impl EvalLogBuilder {
    const NEW_LINE_CHAR: char = '\n';
    const INDENT_SEQ: &'static str = "  ";

    pub fn reset_indent(mut self) -> Self {
        self.indent = 0;
        self
    }

    pub fn inc_indent(mut self) -> Self {
        self.indent += 1;
        self
    }

    pub fn dec_indent(mut self) -> Self {
        self.indent -= 1;
        self
    }

    pub fn new_ln(mut self, message: Option<&str>) -> Self {
        self.content.push(Self::NEW_LINE_CHAR);
        self.content
            .push_str(Self::INDENT_SEQ.repeat(self.indent).as_str());
        if let Some(msg) = message {
            self.content.push_str(msg)
        }
        self
    }

    pub fn append(mut self, val: impl Into<String>) -> Self {
        self.content.push_str(val.into().as_str());
        self
    }

    pub fn append_then_clause(
        mut self,
        new_line: bool,
        result: &ConditionResult,
        rule: &TargetingRule,
    ) -> Self {
        self = self.inc_indent();

        if new_line {
            self = self.new_ln(None);
        } else {
            self = self.append(" ");
        }
        self = self.append("THEN");

        if let Some(sv) = rule.served_value.as_ref() {
            self = self.append(format!("{setting_value}", setting_value = sv.value));
        } else {
            self = self.append(" % options");
        }

        match result {
            ConditionResult::Ok(matched) => {
                if *matched {
                    self = self.append("MATCH, applying rule")
                } else {
                    self = self.append("no match")
                }
            }
            _ => self = self.append(format!("{result}")),
        }

        self
    }

    pub fn content(&self) -> &str {
        self.content.as_str()
    }
}
