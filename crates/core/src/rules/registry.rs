//! 校验器 / 脱敏器注册表：name → 工厂函数。
//!
//! 注册时存 `Box<dyn Fn() -> Box<dyn T>>`，`get(name)` 每次返回全新实例，
//! 避免跨线程共享可变状态。注册表自身可 `Clone` 以便 loader 各自持有副本。

use std::collections::HashMap;
use std::sync::Arc;

use crate::maskers::Masker;
use crate::validators::Validator;

/// 校验器注册表。
#[derive(Clone, Default)]
pub struct ValidatorRegistry {
    factories: HashMap<String, Arc<dyn Fn() -> Box<dyn Validator> + Send + Sync>>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个校验器工厂。重复注册会覆盖同名条目。
    pub fn register<F>(&mut self, name: impl Into<String>, factory: F)
    where
        F: Fn() -> Box<dyn Validator> + Send + Sync + 'static,
    {
        self.factories.insert(name.into(), Arc::new(factory));
    }

    /// 按名取一个全新校验器实例；未注册返回 `None`。
    pub fn get(&self, name: &str) -> Option<Box<dyn Validator>> {
        self.factories.get(name).map(|f| f())
    }

    /// 是否注册了指定名。
    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }
}

/// 脱敏器注册表。
#[derive(Clone, Default)]
pub struct MaskerRegistry {
    factories: HashMap<String, Arc<dyn Fn() -> Box<dyn Masker> + Send + Sync>>,
}

impl MaskerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(&mut self, name: impl Into<String>, factory: F)
    where
        F: Fn() -> Box<dyn Masker> + Send + Sync + 'static,
    {
        self.factories.insert(name.into(), Arc::new(factory));
    }

    pub fn get(&self, name: &str) -> Option<Box<dyn Masker>> {
        self.factories.get(name).map(|f| f())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maskers::Masker;
    use crate::validators::{ValidationResult, Validator};

    struct AlwaysOk;
    impl Validator for AlwaysOk {
        fn validate(&self, _value: &str) -> ValidationResult {
            ValidationResult::ok()
        }
    }

    struct UpperMasker;
    impl Masker for UpperMasker {
        fn mask(&self, value: &str) -> String {
            value.to_uppercase()
        }
    }

    #[test]
    fn validator_registry_register_and_get() {
        let mut reg = ValidatorRegistry::new();
        assert!(!reg.contains("always_ok"));
        reg.register("always_ok", || Box::new(AlwaysOk));
        assert!(reg.contains("always_ok"));
        let v = reg.get("always_ok").expect("registered");
        assert!(v.validate("x").valid);
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn masker_registry_register_and_get() {
        let mut reg = MaskerRegistry::new();
        reg.register("upper", || Box::new(UpperMasker));
        let m = reg.get("upper").expect("registered");
        assert_eq!(m.mask("abc"), "ABC");
        assert!(reg.get("missing").is_none());
    }
}
