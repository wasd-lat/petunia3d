//! Estado puro do controle numérico 3D.
//!
//! O widget Slint apenas apresenta este contrato. Parsing e transições ficam em
//! Rust para que o mesmo comportamento possa ser usado por inspector, painel de
//! operação e testes sem abrir uma janela.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericFieldState {
    value: f32,
    original_value: f32,
    minimum: Option<f32>,
    maximum: Option<f32>,
    step: f32,
    fine_step: f32,
    editing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumericInputError {
    Empty,
    Invalid,
    NonFinite,
}

impl NumericFieldState {
    pub fn new(value: f32, minimum: Option<f32>, maximum: Option<f32>) -> Self {
        let value = sanitize_value(value);
        Self {
            value,
            original_value: value,
            minimum,
            maximum,
            step: 0.01,
            fine_step: 0.001,
            editing: false,
        }
    }

    pub fn with_steps(mut self, step: f32, fine_step: f32) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        if fine_step.is_finite() && fine_step > 0.0 {
            self.fine_step = fine_step;
        }
        self
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32) -> f32 {
        self.value = self.clamp(value);
        self.original_value = self.value;
        self.editing = false;
        self.value
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn begin_edit(&mut self) -> f32 {
        self.original_value = self.value;
        self.editing = true;
        self.value
    }

    pub fn commit_text(&mut self, text: &str) -> Result<f32, NumericInputError> {
        let parsed = parse_numeric(text)?;
        self.value = self.clamp(parsed);
        self.original_value = self.value;
        self.editing = false;
        Ok(self.value)
    }

    pub fn cancel(&mut self) -> f32 {
        self.value = self.original_value;
        self.editing = false;
        self.value
    }

    pub fn scrub(&mut self, horizontal_delta: f32, fine: bool) -> f32 {
        if !horizontal_delta.is_finite() {
            return self.value;
        }
        let step = if fine { self.fine_step } else { self.step };
        self.value = self.clamp(self.value + horizontal_delta * step);
        self.value
    }

    fn clamp(&self, value: f32) -> f32 {
        let value = sanitize_value(value);
        let value = self.minimum.map_or(value, |minimum| value.max(minimum));
        self.maximum.map_or(value, |maximum| value.min(maximum))
    }
}

pub fn parse_numeric(text: &str) -> Result<f32, NumericInputError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(NumericInputError::Empty);
    }
    let value = trimmed
        .parse::<f32>()
        .map_err(|_| NumericInputError::Invalid)?;
    if !value.is_finite() {
        return Err(NumericInputError::NonFinite);
    }
    Ok(value)
}

fn sanitize_value(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_input_commits_and_clamps_to_semantic_range() {
        let mut field = NumericFieldState::new(0.5, Some(0.0), Some(1.0));
        field.begin_edit();
        assert_eq!(field.commit_text(" 1.25 "), Ok(1.0));
        assert!(!field.is_editing());
    }

    #[test]
    fn invalid_input_does_not_corrupt_previous_value() {
        let mut field = NumericFieldState::new(0.5, Some(0.0), Some(1.0));
        field.begin_edit();
        assert_eq!(
            field.commit_text("not-a-number"),
            Err(NumericInputError::Invalid)
        );
        assert_eq!(field.value(), 0.5);
        assert!(field.is_editing());
    }

    #[test]
    fn escape_restores_value_before_edit() {
        let mut field = NumericFieldState::new(2.0, None, None);
        field.begin_edit();
        field.scrub(10.0, false);
        assert_eq!(field.cancel(), 2.0);
        assert!(!field.is_editing());
    }

    #[test]
    fn fine_scrubbing_uses_a_smaller_step() {
        let mut field = NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01);
        assert_eq!(field.scrub(2.0, false), 1.2);
        assert_eq!(field.scrub(2.0, true), 1.22);
    }

    #[test]
    fn setting_a_value_preserves_constraints_and_resets_editing() {
        let mut field = NumericFieldState::new(1.0, Some(0.5), Some(2.0));
        field.begin_edit();

        assert_eq!(field.set_value(4.0), 2.0);
        assert!(!field.is_editing());
        assert_eq!(field.cancel(), 2.0);
    }

    #[test]
    fn parser_rejects_empty_and_non_finite_values() {
        assert_eq!(parse_numeric(""), Err(NumericInputError::Empty));
        assert_eq!(parse_numeric("NaN"), Err(NumericInputError::NonFinite));
        assert_eq!(parse_numeric("Infinity"), Err(NumericInputError::NonFinite));
        assert_eq!(parse_numeric("-0.25"), Ok(-0.25));
    }
}
