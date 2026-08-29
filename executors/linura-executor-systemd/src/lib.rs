#![forbid(unsafe_code)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SystemdOperation {
    SetUnitEnabled { unit: UnitName, enabled: bool },
    RestartUnit { unit: UnitName },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitName(String);

impl UnitName {
    pub fn parse(value: impl Into<String>) -> Result<Self, UnitNameError> {
        let value = value.into();
        if value.is_empty() || value.len() > 255 {
            return Err(UnitNameError::InvalidLength);
        }
        if !value.ends_with(".service") {
            return Err(UnitNameError::UnsupportedUnitType);
        }
        if value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '/' | '\\'))
        {
            return Err(UnitNameError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitNameError {
    InvalidLength,
    UnsupportedUnitType,
    InvalidCharacter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_service_units_are_accepted_initially() {
        assert!(UnitName::parse("sshd.service").is_ok());
        assert_eq!(
            UnitName::parse("multi-user.target"),
            Err(UnitNameError::UnsupportedUnitType)
        );
    }

    #[test]
    fn suspicious_unit_names_are_rejected() {
        assert_eq!(
            UnitName::parse("../../evil.service"),
            Err(UnitNameError::InvalidCharacter)
        );
        assert_eq!(
            UnitName::parse("bad name.service"),
            Err(UnitNameError::InvalidCharacter)
        );
    }
}
