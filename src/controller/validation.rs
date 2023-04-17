use std::fmt::Debug;

use lapin::message::Delivery;
use validator::{validate_email, Validate, ValidationError};

pub fn parse_validate<'a, T: Debug + serde::Deserialize<'a> + Validate>(
    delivery: &'a Delivery,
) -> Result<T, String> {
    let parsed = serde_json::from_slice::<'a, T>(&delivery.data)
        .or_else(|e| Err(format!("parse error: {:#?}", e)))?;

    parsed
        .validate()
        .or_else(|e| Err(format!("validation error: {:#?}", e)))?;

    Ok(parsed)
}

pub fn email_vec(emails: &Vec<String>) -> Result<(), ValidationError> {
    for email in emails {
        if !validate_email(email) {
            return Err(ValidationError::new("vec contains invalid email"));
        }
    }

    Ok(())
}
