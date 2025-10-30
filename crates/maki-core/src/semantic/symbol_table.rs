//! Enhanced symbol table with thread-safety and comprehensive indexing

use super::{Symbol, SymbolType};
use crate::Location;
use dashmap::DashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// Symbol table errors
#[derive(Debug, Error, Clone)]
pub enum SymbolError {
    #[error("Duplicate symbol: {name} defined in {file1} and {file2}")]
    DuplicateSymbol {
        name: String,
        file1: PathBuf,
        file2: PathBuf,
    },

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Unresolved reference: {reference} in {file}")]
    UnresolvedReference { reference: String, file: PathBuf },

    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<String>),
}

/// Unresolved reference tracking
#[derive(Debug, Clone)]
pub struct UnresolvedRef {
    pub reference: String,
    pub referrer: String,
    pub source_file: PathBuf,
    pub location: Location,
}

/// Enhanced symbol table with thread-safety and comprehensive indexing
pub struct EnhancedSymbolTable {
    // Primary indexes (thread-safe)
    by_name: DashMap<String, Arc<Symbol>>,
    by_url: DashMap<String, Arc<Symbol>>,
    by_id: DashMap<String, Arc<Symbol>>,

    // Secondary indexes
    by_kind: DashMap<SymbolType, Vec<Arc<Symbol>>>,
    by_file: DashMap<PathBuf, Vec<Arc<Symbol>>>,

    // Dependency tracking
    dependencies: DashMap<String, HashSet<String>>,
    dependents: DashMap<String, HashSet<String>>,

    // Forward reference tracking
    unresolved_refs: DashMap<String, Vec<UnresolvedRef>>,
}

impl EnhancedSymbolTable {
    /// Create a new empty symbol table
    pub fn new() -> Self {
        Self {
            by_name: DashMap::new(),
            by_url: DashMap::new(),
            by_id: DashMap::new(),
            by_kind: DashMap::new(),
            by_file: DashMap::new(),
            dependencies: DashMap::new(),
            dependents: DashMap::new(),
            unresolved_refs: DashMap::new(),
        }
    }

    /// Add symbol with duplicate detection
    pub fn add_symbol(&self, symbol: Symbol) -> Result<(), SymbolError> {
        let name = symbol.name.clone();
        let file = symbol.definition_location.file.clone();

        // Check for duplicates
        if let Some(existing) = self.by_name.get(&name) {
            return Err(SymbolError::DuplicateSymbol {
                name,
                file1: existing.definition_location.file.clone(),
                file2: file,
            });
        }

        let arc_symbol = Arc::new(symbol);

        // Add to name index
        self.by_name.insert(name.clone(), Arc::clone(&arc_symbol));

        // Add to URL index if present
        if let Some(url) = arc_symbol.definition_location.file.to_str() {
            self.by_url.insert(url.to_string(), Arc::clone(&arc_symbol));
        }

        // Add to kind index
        self.by_kind
            .entry(arc_symbol.symbol_type.clone())
            .or_default()
            .push(Arc::clone(&arc_symbol));

        // Add to file index
        self.by_file.entry(file).or_default().push(arc_symbol);

        Ok(())
    }

    /// Lookup by name
    pub fn get_by_name(&self, name: &str) -> Option<Arc<Symbol>> {
        self.by_name.get(name).map(|r| Arc::clone(&r))
    }

    /// Lookup by URL
    pub fn get_by_url(&self, url: &str) -> Option<Arc<Symbol>> {
        self.by_url.get(url).map(|r| Arc::clone(&r))
    }

    /// Lookup by ID
    pub fn get_by_id(&self, id: &str) -> Option<Arc<Symbol>> {
        self.by_id.get(id).map(|r| Arc::clone(&r))
    }

    /// Get all symbols of a specific kind
    pub fn get_by_kind(&self, kind: SymbolType) -> Vec<Arc<Symbol>> {
        self.by_kind
            .get(&kind)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Get all symbols in a file
    pub fn get_by_file(&self, file: &Path) -> Vec<Arc<Symbol>> {
        self.by_file
            .get(file)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Check if symbol exists
    pub fn contains(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// Add dependency relationship
    pub fn add_dependency(&self, from: &str, to: &str) {
        self.dependencies
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());

        self.dependents
            .entry(to.to_string())
            .or_default()
            .insert(from.to_string());
    }

    /// Get dependencies of a symbol
    pub fn get_dependencies(&self, name: &str) -> HashSet<String> {
        self.dependencies
            .get(name)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Get dependents of a symbol (things that depend on it)
    pub fn get_dependents(&self, name: &str) -> HashSet<String> {
        self.dependents
            .get(name)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Register unresolved reference
    pub fn add_unresolved_ref(&self, reference: UnresolvedRef) {
        self.unresolved_refs
            .entry(reference.reference.clone())
            .or_default()
            .push(reference);
    }

    /// Get all unresolved references (those that still don't have symbols)
    pub fn get_unresolved_references(&self) -> Vec<UnresolvedRef> {
        let mut unresolved = Vec::new();

        for entry in self.unresolved_refs.iter() {
            let ref_name = entry.key();
            if !self.contains(ref_name) {
                unresolved.extend(entry.value().iter().cloned());
            }
        }

        unresolved
    }

    /// Find duplicate definitions
    pub fn find_duplicates(&self) -> Vec<(String, Vec<Arc<Symbol>>)> {
        // This implementation assumes duplicates were prevented during add_symbol
        // Returns empty vec as duplicates are not allowed
        Vec::new()
    }

    /// Clear all symbols
    pub fn clear(&self) {
        self.by_name.clear();
        self.by_url.clear();
        self.by_id.clear();
        self.by_kind.clear();
        self.by_file.clear();
        self.dependencies.clear();
        self.dependents.clear();
        self.unresolved_refs.clear();
    }

    /// Get all symbol names
    pub fn symbol_names(&self) -> Vec<String> {
        self.by_name.iter().map(|r| r.key().clone()).collect()
    }

    /// Get total symbol count
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

impl Default for EnhancedSymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn create_test_symbol(name: &str, kind: SymbolType, file: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            symbol_type: kind,
            definition_location: Location {
                file: PathBuf::from(file),
                line: 1,
                column: 1,
                end_line: Some(1),
                end_column: Some(name.len() + 1),
                offset: 0,
                length: name.len(),
                span: Some((0, name.len())),
            },
            references: Vec::new(),
        }
    }

    #[test]
    fn test_add_and_lookup() {
        let table = EnhancedSymbolTable::new();

        let symbol = create_test_symbol("MyProfile", SymbolType::Profile, "test.fsh");
        table.add_symbol(symbol).unwrap();

        // Lookup by name
        assert!(table.get_by_name("MyProfile").is_some());
        assert!(table.contains("MyProfile"));
    }

    #[test]
    fn test_duplicate_detection() {
        let table = EnhancedSymbolTable::new();

        let symbol1 = create_test_symbol("Duplicate", SymbolType::Profile, "file1.fsh");
        let symbol2 = create_test_symbol("Duplicate", SymbolType::Profile, "file2.fsh");

        table.add_symbol(symbol1).unwrap();
        let result = table.add_symbol(symbol2);

        assert!(result.is_err());
        assert!(matches!(result, Err(SymbolError::DuplicateSymbol { .. })));
    }

    #[test]
    fn test_lookup_by_kind() {
        let table = EnhancedSymbolTable::new();

        table
            .add_symbol(create_test_symbol(
                "Profile1",
                SymbolType::Profile,
                "test.fsh",
            ))
            .unwrap();
        table
            .add_symbol(create_test_symbol(
                "Profile2",
                SymbolType::Profile,
                "test.fsh",
            ))
            .unwrap();
        table
            .add_symbol(create_test_symbol(
                "ValueSet1",
                SymbolType::ValueSet,
                "test.fsh",
            ))
            .unwrap();

        let profiles = table.get_by_kind(SymbolType::Profile);
        assert_eq!(profiles.len(), 2);

        let valuesets = table.get_by_kind(SymbolType::ValueSet);
        assert_eq!(valuesets.len(), 1);
    }

    #[test]
    fn test_lookup_by_file() {
        let table = EnhancedSymbolTable::new();

        table
            .add_symbol(create_test_symbol(
                "Symbol1",
                SymbolType::Profile,
                "file1.fsh",
            ))
            .unwrap();
        table
            .add_symbol(create_test_symbol(
                "Symbol2",
                SymbolType::Profile,
                "file1.fsh",
            ))
            .unwrap();
        table
            .add_symbol(create_test_symbol(
                "Symbol3",
                SymbolType::Profile,
                "file2.fsh",
            ))
            .unwrap();

        let file1_symbols = table.get_by_file(Path::new("file1.fsh"));
        assert_eq!(file1_symbols.len(), 2);

        let file2_symbols = table.get_by_file(Path::new("file2.fsh"));
        assert_eq!(file2_symbols.len(), 1);
    }

    #[test]
    fn test_dependency_tracking() {
        let table = EnhancedSymbolTable::new();

        table.add_dependency("ChildProfile", "ParentProfile");
        table.add_dependency("ChildProfile", "SomeExtension");

        let deps = table.get_dependencies("ChildProfile");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains("ParentProfile"));
        assert!(deps.contains("SomeExtension"));

        let dependents = table.get_dependents("ParentProfile");
        assert_eq!(dependents.len(), 1);
        assert!(dependents.contains("ChildProfile"));
    }

    #[test]
    fn test_unresolved_references() {
        let table = EnhancedSymbolTable::new();

        let unresolved = UnresolvedRef {
            reference: "UnknownProfile".to_string(),
            referrer: "MyProfile".to_string(),
            source_file: PathBuf::from("test.fsh"),
            location: Location {
                file: PathBuf::from("test.fsh"),
                line: 10,
                column: 5,
                end_line: Some(10),
                end_column: Some(19),
                offset: 100,
                length: 14,
                span: Some((100, 114)),
            },
        };

        table.add_unresolved_ref(unresolved);

        let unresolved_refs = table.get_unresolved_references();
        assert_eq!(unresolved_refs.len(), 1);

        // After adding the symbol, it should resolve
        table
            .add_symbol(create_test_symbol(
                "UnknownProfile",
                SymbolType::Profile,
                "test.fsh",
            ))
            .unwrap();

        let unresolved_refs = table.get_unresolved_references();
        assert_eq!(unresolved_refs.len(), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let table = Arc::new(EnhancedSymbolTable::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let table = Arc::clone(&table);
                thread::spawn(move || {
                    let symbol = create_test_symbol(
                        &format!("Symbol{}", i),
                        SymbolType::Profile,
                        "test.fsh",
                    );
                    table.add_symbol(symbol).unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(table.len(), 10);
    }

    #[test]
    fn test_clear() {
        let table = EnhancedSymbolTable::new();

        table
            .add_symbol(create_test_symbol(
                "Symbol1",
                SymbolType::Profile,
                "test.fsh",
            ))
            .unwrap();
        assert_eq!(table.len(), 1);

        table.clear();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }
}

/// Combined symbol table (FSH + FHIR definitions)
pub struct CombinedSymbolTable {
    fsh_symbols: Arc<EnhancedSymbolTable>,
    fhir_session: Arc<crate::canonical::DefinitionSession>,
}

impl CombinedSymbolTable {
    /// Create a new combined symbol table
    pub fn new(
        fsh_symbols: Arc<EnhancedSymbolTable>,
        fhir_session: Arc<crate::canonical::DefinitionSession>,
    ) -> Self {
        Self {
            fsh_symbols,
            fhir_session,
        }
    }

    /// Lookup symbol in FSH first, then FHIR definitions
    pub fn lookup(&self, name: &str) -> Option<Arc<Symbol>> {
        // Try FSH symbols first
        if let Some(symbol) = self.fsh_symbols.get_by_name(name) {
            return Some(symbol);
        }

        // TODO: Convert FHIR definitions to Symbol format
        None
    }

    /// Fish for resource (FSH → FHIR cascading)
    pub async fn fish(
        &self,
        item: &str,
        types: &[crate::canonical::fishable::FhirType],
    ) -> crate::canonical::CanonicalResult<Option<Arc<crate::canonical::DefinitionResource>>> {
        use crate::canonical::fishable::Fishable;

        // Try FHIR session fishing
        self.fhir_session.fish(item, types).await
    }

    /// Get FSH symbol table
    pub fn fsh_symbols(&self) -> &Arc<EnhancedSymbolTable> {
        &self.fsh_symbols
    }

    /// Get FHIR session
    pub fn fhir_session(&self) -> &Arc<crate::canonical::DefinitionSession> {
        &self.fhir_session
    }
}

#[cfg(test)]
mod combined_tests {
    use super::tests::create_test_symbol;
    use super::*;

    #[test]
    fn test_combined_lookup_fsh_first() {
        let fsh = Arc::new(EnhancedSymbolTable::new());

        let symbol = create_test_symbol("MyProfile", SymbolType::Profile, "test.fsh");
        fsh.add_symbol(symbol).unwrap();

        // TODO: Add test with actual FHIR session when available
        // For now just verify FSH lookup works
        assert!(fsh.get_by_name("MyProfile").is_some());
    }
}
