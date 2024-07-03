use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

/// Concurrently safe registry for storing state.
///
/// Must be built using the ['StateRegistryBuilder'].
/// All providers must be registered by the builder.
pub struct StateRegistry(HashMap<TypeId, Box<dyn Any + Send + Sync>>);

impl StateRegistry {
    ///Get a type from the StateRegistry
    ///
    /// #Returns
    /// Some(&T) - returns a reference to the value associated with the key if present.
    /// None - if there was no value associated with the key.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.0.get(&TypeId::of::<T>()).map(|state| state.downcast_ref())?
    }
}

/// Builder for the ['StateRegistry']
/// The builder is used to insert state providers into the registry.
/// Once built, items cannot be inserted into the registry.
///
/// #Example
/// ```
/// use phylax_interfaces::state_registry::{StateRegistryBuilder, StateRegistry};
///
/// #[derive(Debug, PartialEq, Eq)]
/// pub struct MockBackend;
///
/// let mut state_registry_builder = StateRegistryBuilder::default();
/// state_registry_builder.insert(MockBackend);
/// let state_registry = state_registry_builder.build();
///
/// assert_eq!(state_registry.get::<MockBackend>(), Some(&MockBackend));
/// ```
pub struct StateRegistryBuilder(StateRegistry);
impl StateRegistryBuilder {
    /// Inserts a state provider into the registry.
    ///
    /// #Returns
    /// Some(prev_value) - returns the previous value associated with the key if present.
    /// None - if there was no value associated with the key.
    pub fn insert<T: Sync + Send + 'static>(
        &mut self,
        state: T,
    ) -> Option<Box<dyn Any + Send + Sync>> {
        self.0 .0.insert(TypeId::of::<T>(), Box::new(state))
    }

    /// Builds the ['StateRegistry'] from the builder.
    pub fn build(self) -> StateRegistry {
        self.0
    }
}

impl Default for StateRegistryBuilder {
    fn default() -> Self {
        Self(StateRegistry(HashMap::new()))
    }
}

#[test]
fn test_state_registry() {
    use std::sync::Arc;
    // Build the state registry
    let mut state_registry_builder = StateRegistryBuilder::default();

    // Insert a state provider
    assert!(state_registry_builder.insert(Arc::new("TestA")).is_none());

    let previous_value = state_registry_builder.insert(Arc::new("TestB"));
    assert!(previous_value.is_some());
    assert_eq!(previous_value.unwrap().downcast_ref::<Arc<&str>>(), Some(&Arc::new("TestA")));

    let state_registry = state_registry_builder.build();

    assert_eq!(state_registry.get::<Arc<&str>>(), Some(&Arc::new("TestB")));
}
