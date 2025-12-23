// ai generated function
pub fn check_email(email: &str) -> bool {
    // Must contain exactly one '@'
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    // Local and domain parts must not be empty
    if local.is_empty() || domain.is_empty() {
        return false;
    }

    // Domain must contain at least one dot
    let domain_parts: Vec<&str> = domain.split('.').collect();
    if domain_parts.len() < 2 {
        return false;
    }

    // No empty sections in the domain (e.g. "example..com")
    if domain_parts.iter().any(|part| part.is_empty()) {
        return false;
    }

    // No spaces allowed
    if email.contains(' ') {
        return false;
    }

    true
}
