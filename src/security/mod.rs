#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutation {
    ReadOnly,
    Mutating,
    Destructive,
}

#[derive(Debug, Clone, Copy)]
pub struct SafetyPolicy {
    safe_mode: bool,
    read_only: bool,
}

impl SafetyPolicy {
    pub fn new(safe_mode: bool, read_only: bool) -> Self {
        Self {
            safe_mode,
            read_only,
        }
    }

    pub fn allows(&self, mutation: Mutation) -> bool {
        !self.read_only || mutation == Mutation::ReadOnly
    }

    pub fn requires_confirmation(&self, mutation: Mutation) -> bool {
        mutation == Mutation::Destructive || (self.safe_mode && mutation == Mutation::Mutating)
    }
}

#[cfg(test)]
mod tests {
    use super::{Mutation, SafetyPolicy};

    #[test]
    fn read_only_policy_blocks_mutations_but_allows_queries() {
        let policy = SafetyPolicy::new(true, true);

        assert!(policy.allows(Mutation::ReadOnly));
        assert!(!policy.allows(Mutation::Mutating));
        assert!(!policy.allows(Mutation::Destructive));
    }

    #[test]
    fn safe_mode_confirms_mutations_and_always_confirms_destructive_actions() {
        let safe = SafetyPolicy::new(true, false);
        let unsafe_policy = SafetyPolicy::new(false, false);

        assert!(safe.requires_confirmation(Mutation::Mutating));
        assert!(safe.requires_confirmation(Mutation::Destructive));
        assert!(!unsafe_policy.requires_confirmation(Mutation::Mutating));
        assert!(unsafe_policy.requires_confirmation(Mutation::Destructive));
    }
}
