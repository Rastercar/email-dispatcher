use validator::{validate_email, ValidationError};

pub fn email_vec(emails: &Vec<String>) -> Result<(), ValidationError> {
    for email in emails {
        if !validate_email(email) {
            return Err(ValidationError::new("vec contains invalid email"));
        }
    }

    Ok(())
}
