//! Instance Exporter
//!
//! Exports FSH Instance definitions to FHIR resource instances (JSON).
//!
//! # Overview
//!
//! Instances are concrete examples of FHIR resources with specific data values.
//! This module handles:
//! - Converting FSH assignment rules (* path = value) to JSON
//! - Nested path navigation (e.g., address[0].line[+])
//! - Array indexing ([0], [+], [=])
//! - Value type conversion (strings, codes, references, etc.)
//!
//! # Algorithm
//!
//! Based on Algorithm 7 from MAKI_PLAN.md:
//! 1. Create base resource JSON with resourceType and id
//! 2. Process each assignment rule sequentially
//! 3. Parse paths and navigate/create nested structures
//! 4. Handle arrays with special indices
//! 5. Convert FSH values to appropriate JSON types
//! 6. Validate the resulting instance
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::InstanceExporter;
//! use maki_core::cst::ast::Instance;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let session: Arc<maki_core::canonical::DefinitionSession> = todo!();
//! // Parse FSH instance
//! let instance: Instance = todo!();
//!
//! // Create exporter
//! let exporter = InstanceExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//! ).await?;
//!
//! // Export to FHIR JSON
//! let resource = exporter.export(&instance).await?;
//!
//! // Serialize
//! let json = serde_json::to_string_pretty(&resource)?;
//! println!("{}", json);
//! # Ok(())
//! # }
//! ```

use super::ExportError;
use super::fhir_types::{ElementDefinition, StructureDefinition};
use crate::canonical::DefinitionSession;
use crate::cst::ast::{FixedValueRule, Instance, Rule};
use crate::semantic::FishingContext;
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

// ============================================================================
// Types
// ============================================================================

/// Array index type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayIndex {
    /// Specific numeric index: [0], [1], [2]
    Numeric(usize),
    /// Append to array: [+]
    Append,
    /// Reference current element: [=]
    Current,
}

/// Path segment (field name or array access)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// Field name
    Field(String),
    /// Array access with index and optional slice name
    ArrayAccess {
        field: String,
        index: ArrayIndex,
        /// Optional slice name for named slices (e.g., extension[myExtension])
        slice_name: Option<String>,
    },
}

// ============================================================================
// Instance Exporter
// ============================================================================

/// Exports FSH Instance definitions to FHIR resource instances
///
/// # Example
///
/// ```rust,no_run
/// use maki_core::export::InstanceExporter;
/// use maki_core::canonical::DefinitionSession;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let session: Arc<DefinitionSession> = todo!();
/// let exporter = InstanceExporter::new(
///     session,
///     "http://example.org/fhir".to_string(),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct InstanceExporter {
    /// Session for resolving FHIR definitions
    #[allow(dead_code)]
    session: Arc<DefinitionSession>,
    /// Fishing context for reference resolution
    fishing_context: Option<Arc<FishingContext>>,
    /// Base URL for instance canonical URLs (if needed)
    #[allow(dead_code)]
    base_url: String,
    /// Track current array indices for [=] operator
    current_indices: HashMap<String, usize>,
    /// Registry of exported instances for reference resolution
    instance_registry: HashMap<String, JsonValue>,
    /// Current profile name (InstanceOf) being exported
    current_profile_name: Option<String>,
    /// Current profile canonical URL (if resolved)
    current_profile_url: Option<String>,
    /// Cached map of extension slice names -> canonical URLs for current profile
    current_extension_urls: HashMap<String, String>,
}

impl InstanceExporter {
    /// Create a new instance exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving resource types and profiles
    /// * `base_url` - Base URL for generated instance identifiers
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::InstanceExporter;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = InstanceExporter::new(
    ///     session,
    ///     "http://example.org/fhir".to_string(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        session: Arc<DefinitionSession>,
        base_url: String,
    ) -> Result<Self, ExportError> {
        Ok(Self {
            session,
            fishing_context: None,
            base_url,
            current_indices: HashMap::new(),
            instance_registry: HashMap::new(),
            current_profile_name: None,
            current_profile_url: None,
            current_extension_urls: HashMap::new(),
        })
    }

    /// Set the fishing context for reference validation
    ///
    /// This enables validation of references to profiles, value sets, and other resources
    /// during instance export.
    pub fn with_fishing_context(mut self, fishing_context: Arc<FishingContext>) -> Self {
        self.fishing_context = Some(fishing_context);
        self
    }

    /// Register an instance for reference resolution
    pub fn register_instance(&mut self, name: String, json: JsonValue) {
        self.instance_registry.insert(name, json);
    }

    /// Get a registered instance by name
    pub fn get_instance(&self, name: &str) -> Option<&JsonValue> {
        self.instance_registry.get(name)
    }

    /// Resolve the base FHIR resource type for a profile by following the parent chain
    ///
    /// This method recursively follows the parent chain of a profile until it finds
    /// a base FHIR resource type (like "Patient", "Observation", etc.).
    ///
    /// # Arguments
    ///
    /// * `fishing_ctx` - Fishing context for looking up profiles
    /// * `profile_name` - Name of the profile to resolve
    /// * `metadata` - Initial metadata for the profile
    ///
    /// # Returns
    ///
    /// The base FHIR resource type (e.g., "Patient" for a CancerPatient profile)
    async fn resolve_base_resource_type(
        &self,
        fishing_ctx: &Arc<FishingContext>,
        profile_name: &str,
        metadata: &crate::semantic::FishableMetadata,
    ) -> String {
        const MAX_DEPTH: usize = 10; // Prevent infinite loops
        let mut depth = 0;
        let mut current_parent = metadata.parent.clone();

        trace!(
            "Starting base resource type resolution for profile '{}' with parent: {:?}",
            profile_name, current_parent
        );

        // Follow the parent chain until we find a base resource or hit max depth
        while let Some(parent) = current_parent {
            depth += 1;
            if depth > MAX_DEPTH {
                warn!(
                    "Max depth ({}) exceeded resolving base type for '{}', using parent '{}'",
                    MAX_DEPTH, profile_name, parent
                );
                return parent;
            }

            trace!("  [Depth {}] Checking parent: '{}'", depth, parent);

            // Check if parent is a known base FHIR resource type
            // Base resources typically don't have parents or their parent is "DomainResource"/"Resource"
            if self.is_base_resource_type(&parent) {
                debug!(
                    "Found base resource type '{}' for profile '{}' at depth {}",
                    parent, profile_name, depth
                );
                return parent;
            }

            // Try to fish for the parent profile's metadata
            if let Some(parent_metadata) = fishing_ctx
                .fish_metadata(
                    &parent,
                    &[
                        crate::semantic::ResourceType::Profile,
                        crate::semantic::ResourceType::Logical,
                    ],
                )
                .await
            {
                trace!(
                    "  [Depth {}] Found parent '{}' with parent: {:?}",
                    depth, parent, parent_metadata.parent
                );
                current_parent = parent_metadata.parent.clone();
            } else {
                // Parent not found in tank, try canonical packages
                trace!(
                    "  [Depth {}] Parent '{}' not in tank, trying canonical",
                    depth, parent
                );

                // Try to resolve alias first (e.g., "USCorePatient" -> canonical URL)
                let parent_to_fish = fishing_ctx
                    .resolve_alias(&parent)
                    .unwrap_or_else(|| parent.clone());
                if parent_to_fish != parent {
                    trace!(
                        "  [Depth {}] Resolved alias '{}' -> '{}'",
                        depth, parent, parent_to_fish
                    );
                }

                match fishing_ctx.fish_structure_definition(&parent_to_fish).await {
                    Ok(Some(sd)) => {
                        trace!(
                            "  [Depth {}] Found parent '{}' in canonical with type: '{}'",
                            depth, parent, sd.type_field
                        );
                        // StructureDefinition.type is the base resource type
                        return sd.type_field;
                    }
                    Ok(None) => {
                        debug!(
                            "Parent '{}' not found (None), assuming it's the base resource type for '{}'",
                            parent, profile_name
                        );
                        return parent;
                    }
                    Err(e) => {
                        debug!(
                            "Parent '{}' error: {}, assuming it's the base resource type for '{}'",
                            parent, e, profile_name
                        );
                        return parent;
                    }
                }
            }
        }

        // No parent found - use the profile name as the base type
        debug!(
            "No parent found for '{}', using profile name as base resource type",
            profile_name
        );
        profile_name.to_string()
    }

    /// Check if a name is a known base FHIR resource type
    ///
    /// This checks against common FHIR resource types to determine if we've
    /// reached the base of the profile inheritance chain.
    fn is_base_resource_type(&self, name: &str) -> bool {
        // Common FHIR R4 resource types
        // See: http://hl7.org/fhir/R4/resourcelist.html
        matches!(
            name,
            "Account"
                | "ActivityDefinition"
                | "AdverseEvent"
                | "AllergyIntolerance"
                | "Appointment"
                | "AppointmentResponse"
                | "AuditEvent"
                | "Basic"
                | "Binary"
                | "BiologicallyDerivedProduct"
                | "BodyStructure"
                | "Bundle"
                | "CapabilityStatement"
                | "CarePlan"
                | "CareTeam"
                | "CatalogEntry"
                | "ChargeItem"
                | "ChargeItemDefinition"
                | "Claim"
                | "ClaimResponse"
                | "ClinicalImpression"
                | "CodeSystem"
                | "Communication"
                | "CommunicationRequest"
                | "CompartmentDefinition"
                | "Composition"
                | "ConceptMap"
                | "Condition"
                | "Consent"
                | "Contract"
                | "Coverage"
                | "CoverageEligibilityRequest"
                | "CoverageEligibilityResponse"
                | "DetectedIssue"
                | "Device"
                | "DeviceDefinition"
                | "DeviceMetric"
                | "DeviceRequest"
                | "DeviceUseStatement"
                | "DiagnosticReport"
                | "DocumentManifest"
                | "DocumentReference"
                | "DomainResource"
                | "EffectEvidenceSynthesis"
                | "Encounter"
                | "Endpoint"
                | "EnrollmentRequest"
                | "EnrollmentResponse"
                | "EpisodeOfCare"
                | "EventDefinition"
                | "Evidence"
                | "EvidenceVariable"
                | "ExampleScenario"
                | "ExplanationOfBenefit"
                | "FamilyMemberHistory"
                | "Flag"
                | "Goal"
                | "GraphDefinition"
                | "Group"
                | "GuidanceResponse"
                | "HealthcareService"
                | "ImagingStudy"
                | "Immunization"
                | "ImmunizationEvaluation"
                | "ImmunizationRecommendation"
                | "ImplementationGuide"
                | "InsurancePlan"
                | "Invoice"
                | "Library"
                | "Linkage"
                | "List"
                | "Location"
                | "Measure"
                | "MeasureReport"
                | "Media"
                | "Medication"
                | "MedicationAdministration"
                | "MedicationDispense"
                | "MedicationKnowledge"
                | "MedicationRequest"
                | "MedicationStatement"
                | "MedicinalProduct"
                | "MedicinalProductAuthorization"
                | "MedicinalProductContraindication"
                | "MedicinalProductIndication"
                | "MedicinalProductIngredient"
                | "MedicinalProductInteraction"
                | "MedicinalProductManufactured"
                | "MedicinalProductPackaged"
                | "MedicinalProductPharmaceutical"
                | "MedicinalProductUndesirableEffect"
                | "MessageDefinition"
                | "MessageHeader"
                | "MolecularSequence"
                | "NamingSystem"
                | "NutritionOrder"
                | "Observation"
                | "ObservationDefinition"
                | "OperationDefinition"
                | "OperationOutcome"
                | "Organization"
                | "OrganizationAffiliation"
                | "Parameters"
                | "Patient"
                | "PaymentNotice"
                | "PaymentReconciliation"
                | "Person"
                | "PlanDefinition"
                | "Practitioner"
                | "PractitionerRole"
                | "Procedure"
                | "Provenance"
                | "Questionnaire"
                | "QuestionnaireResponse"
                | "RelatedPerson"
                | "RequestGroup"
                | "ResearchDefinition"
                | "ResearchElementDefinition"
                | "ResearchStudy"
                | "ResearchSubject"
                | "Resource"
                | "RiskAssessment"
                | "RiskEvidenceSynthesis"
                | "Schedule"
                | "SearchParameter"
                | "ServiceRequest"
                | "Slot"
                | "Specimen"
                | "SpecimenDefinition"
                | "StructureDefinition"
                | "StructureMap"
                | "Subscription"
                | "Substance"
                | "SubstanceNucleicAcid"
                | "SubstancePolymer"
                | "SubstanceProtein"
                | "SubstanceReferenceInformation"
                | "SubstanceSourceMaterial"
                | "SubstanceSpecification"
                | "SupplyDelivery"
                | "SupplyRequest"
                | "Task"
                | "TerminologyCapabilities"
                | "TestReport"
                | "TestScript"
                | "ValueSet"
                | "VerificationResult"
                | "VisionPrescription"
        )
    }

    /// Export an Instance to a FHIR resource (JSON)
    ///
    /// # Arguments
    ///
    /// * `instance` - FSH Instance AST node
    ///
    /// # Returns
    ///
    /// A FHIR resource as JSON
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - InstanceOf type not found
    /// - Rule application fails
    /// - Path resolution fails
    /// - Value conversion fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::InstanceExporter;
    /// use maki_core::cst::ast::Instance;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut exporter: InstanceExporter = todo!();
    /// # let instance: Instance = todo!();
    /// let resource = exporter.export(&instance).await?;
    /// println!("{}", serde_json::to_string_pretty(&resource)?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export(&mut self, instance: &Instance) -> Result<JsonValue, ExportError> {
        let name = instance.name().unwrap_or_else(|| "unnamed".to_string());
        debug!("Exporting instance: {}", name);

        // Get resource type from InstanceOf
        let instance_of = instance
            .instance_of()
            .and_then(|c| c.value())
            .ok_or_else(|| ExportError::MissingRequiredField("InstanceOf".to_string()))?;

        trace!("Instance {} is of type {}", name, instance_of);

        // If InstanceOf references a base FHIR resource type, short-circuit resolution
        // to avoid accidentally selecting similarly named profiles or extensions.
        if self.is_base_resource_type(&instance_of) {
            let mut resource = serde_json::json!({
                "resourceType": instance_of,
                "id": name,
            });

            for rule in instance.rules() {
                self.apply_rule(&mut resource, &rule).await?;
            }

            self.current_profile_name = None;
            self.current_profile_url = None;
            self.current_extension_urls.clear();

            debug!(
                "Exported base resource instance {} with resourceType {}",
                name, instance_of
            );
            return Ok(resource);
        }

        // Resolve profile to get base resource type and canonical URL
        let (resource_type, canonical_url) = if let Some(fishing_ctx) = &self.fishing_context {
            // Try to get metadata from the tank first (profiles not yet exported)
            // This is fast and doesn't require full export
            if let Some(metadata) = fishing_ctx
                .fish_metadata(
                    &instance_of,
                    &[
                        crate::semantic::ResourceType::Profile,
                        crate::semantic::ResourceType::Extension,
                        crate::semantic::ResourceType::Logical,
                    ],
                )
                .await
            {
                // Found in tank - resolve base resource type by following parent chain
                let base_type = self
                    .resolve_base_resource_type(fishing_ctx, &instance_of, &metadata)
                    .await;
                let profile_url = match metadata.resource_type.as_str() {
                    "StructureDefinition" => Some(format!(
                        "{}/StructureDefinition/{}",
                        self.base_url, metadata.id
                    )),
                    _ => Some(metadata.url.clone()),
                };
                debug!(
                    "Resolved profile '{}' from tank -> base type: '{}', canonical URL: '{}'",
                    instance_of,
                    base_type,
                    profile_url.as_deref().unwrap_or("<none>")
                );
                (base_type, profile_url)
            } else {
                // Not in tank, try canonical packages (external FHIR definitions)
                match fishing_ctx.fish_structure_definition(&instance_of).await {
                    Ok(Some(sd)) => {
                        // Found in canonical packages - use its base type and canonical URL
                        let base_type = sd.type_field.clone();
                        let profile_url = sd.url.clone();
                        debug!(
                            "Resolved profile '{}' from canonical -> base type: '{}', canonical URL: '{}'",
                            instance_of, base_type, profile_url
                        );
                        (base_type, Some(profile_url))
                    }
                    Ok(None) => {
                        // Not found anywhere - might be a base resource type
                        debug!(
                            "Profile '{}' not found, assuming it's a base resource type",
                            instance_of
                        );
                        (instance_of.to_string(), None)
                    }
                    Err(e) => {
                        // Error during resolution - log warning and fall back
                        warn!(
                            "Error resolving profile '{}': {}, using as resourceType",
                            instance_of, e
                        );
                        (instance_of.to_string(), None)
                    }
                }
            }
        } else {
            // No fishing context available - fall back to using instance_of as-is
            debug!(
                "No fishing context available, using '{}' as resourceType",
                instance_of
            );
            (instance_of.to_string(), None)
        };

        // Optionally load full StructureDefinition to refine resourceType/canonical and extract slices
        let sd_opt = if let Some(fishing_ctx) = &self.fishing_context {
            self.load_structure_definition(fishing_ctx, &instance_of, canonical_url.as_deref())
                .await
        } else {
            None
        };

        let mut resource_type = resource_type;
        let mut canonical_url = canonical_url;

        if let Some(sd) = sd_opt.as_ref() {
            if canonical_url.is_none() {
                canonical_url = Some(sd.url.clone());
            }
            // Ensure resourceType matches the base type, not the profile name
            resource_type = sd.type_field.clone();
        }

        // Cache profile context for this export (used by extension resolution)
        self.current_profile_name = Some(instance_of.to_string());
        self.current_profile_url = canonical_url.clone();
        self.current_extension_urls = sd_opt
            .as_ref()
            .map(Self::build_extension_url_map)
            .unwrap_or_default();

        // Create base resource with correct resourceType
        let mut resource = if let Some(profile_url) = canonical_url {
            // Include meta.profile with canonical URL
            serde_json::json!({
                "resourceType": resource_type,
                "id": name,
                "meta": {
                    "profile": [profile_url]
                }
            })
        } else {
            // No profile URL available - just use resourceType and id
            serde_json::json!({
                "resourceType": resource_type,
                "id": name,
            })
        };

        // Debug: print base resource
        if name.contains("genomic-variant-fusion") {
            eprintln!(
                "[DEBUG INSTANCE EXPORT] Base resource for {}: {}",
                name,
                serde_json::to_string_pretty(&resource).unwrap_or_default()
            );
        }

        // Apply rules
        for rule in instance.rules() {
            self.apply_rule(&mut resource, &rule).await?;
        }

        // Debug: print final resource
        if name.contains("genomic-variant-fusion") {
            eprintln!(
                "[DEBUG INSTANCE EXPORT] Final resource for {}: {}",
                name,
                serde_json::to_string_pretty(&resource).unwrap_or_default()
            );
        }

        // Clear per-instance caches
        self.current_profile_name = None;
        self.current_profile_url = None;
        self.current_extension_urls.clear();

        debug!("Successfully exported instance {}", name);
        Ok(resource)
    }

    /// Apply a single rule to the resource
    async fn apply_rule(
        &mut self,
        resource: &mut JsonValue,
        rule: &Rule,
    ) -> Result<(), ExportError> {
        match rule {
            Rule::FixedValue(fixed_rule) => {
                self.apply_fixed_value_rule(resource, fixed_rule).await?;
            }
            Rule::Card(_) => {
                // Card rules don't apply to instances
                trace!("Skipping card rule in instance");
            }
            Rule::Flag(_) => {
                // Flag rules don't apply to instances
                trace!("Skipping flag rule in instance");
            }
            Rule::ValueSet(_) => {
                // ValueSet rules don't apply to instances
                trace!("Skipping valueset rule in instance");
            }
            Rule::Path(_) => {
                // Path rules don't apply to instances
                trace!("Skipping path rule in instance");
            }
            Rule::AddElement(_)
            | Rule::Contains(_)
            | Rule::Only(_)
            | Rule::Obeys(_)
            | Rule::Mapping(_)
            | Rule::CaretValue(_)
            | Rule::CodeCaretValue(_)
            | Rule::CodeInsert(_) => {
                // These rules don't apply to instances
                trace!("Skipping contains/only/obeys rule in instance");
            }
        }
        Ok(())
    }

    /// Apply a fixed value rule (assignment: * path = value)
    async fn apply_fixed_value_rule(
        &mut self,
        resource: &mut JsonValue,
        rule: &FixedValueRule,
    ) -> Result<(), ExportError> {
        // Get path and value
        let path = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::InvalidPath {
                path: "<unknown>".to_string(),
                resource: resource["resourceType"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
            })?;

        // Skip if there's no value - this is actually a path rule (structural navigation)
        // Example: * valueCodeableConcept (without = value)
        let Some(value_str) = rule.value() else {
            trace!(
                "Skipping fixed value rule with no value (path rule): {}",
                path
            );
            return Ok(());
        };

        trace!("Applying assignment: {} = {}", path, value_str);

        // Parse the path into segments
        let segments = self.parse_path(&path)?;

        // Convert value string to JSON, passing path context to preserve string types
        let json_value = self.convert_value_with_path(&value_str, &path).await?;

        // Navigate and set the value
        self.set_value_at_path(resource, &segments, json_value)
            .await?;

        Ok(())
    }

    /// Parse a path string into segments
    ///
    /// Examples:
    /// - "name.family" -> [Field("name"), Field("family")]
    /// - "name.given[0]" -> [Field("name"), ArrayAccess("given", 0, None)]
    /// - "address[+].line[0]" -> [ArrayAccess("address", Append, None), ArrayAccess("line", 0, None)]
    /// - "extension[myExt].valueString" -> [ArrayAccess("extension", 0, Some("myExt")), Field("valueString")]
    fn parse_path(&self, path: &str) -> Result<Vec<PathSegment>, ExportError> {
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut chars = path.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Field(current.clone()));
                        current.clear();
                    }
                }
                '[' => {
                    // Parse array index or slice name
                    let field = current.clone();
                    current.clear();

                    let mut index_str = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == ']' {
                            chars.next(); // consume ']'
                            break;
                        }
                        index_str.push(chars.next().unwrap());
                    }

                    let index_str_trimmed = index_str.trim();

                    // Determine if it's a slice name or numeric index
                    let (index, slice_name) = if index_str_trimmed == "+" {
                        (ArrayIndex::Append, None)
                    } else if index_str_trimmed == "=" {
                        (ArrayIndex::Current, None)
                    } else if let Ok(num) = index_str_trimmed.parse::<usize>() {
                        (ArrayIndex::Numeric(num), None)
                    } else {
                        // It's a slice name - use index 0 and store the name
                        (ArrayIndex::Numeric(0), Some(index_str_trimmed.to_string()))
                    };

                    // Handle consecutive brackets like [sliceName][0]
                    // If the field is empty and the previous segment is an ArrayAccess,
                    // this is a secondary index on the same slice
                    if field.is_empty() {
                        if let Some(PathSegment::ArrayAccess {
                            field: prev_field,
                            slice_name: prev_slice,
                            ..
                        }) = segments.last()
                        {
                            // Update the previous segment with the new index while keeping the slice name
                            let updated_segment = PathSegment::ArrayAccess {
                                field: prev_field.clone(),
                                index: index.clone(),
                                slice_name: prev_slice.clone(),
                            };
                            segments.pop();
                            segments.push(updated_segment);
                            continue;
                        }
                    }

                    segments.push(PathSegment::ArrayAccess {
                        field,
                        index,
                        slice_name,
                    });
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        // Add final segment if any
        if !current.is_empty() {
            segments.push(PathSegment::Field(current));
        }

        if segments.is_empty() {
            return Err(ExportError::InvalidPath {
                path: path.to_string(),
                resource: "".to_string(),
            });
        }

        Ok(segments)
    }

    /// Set a value at the given path, creating intermediate structures as needed
    ///
    /// This method handles:
    /// - Simple field assignment (name.family = "Doe")
    /// - Array indexing (name.given[0] = "John")
    /// - Slice names (extension[myExtension].url = "http://...")
    /// - Complex value merging (when setting properties on existing objects)
    async fn set_value_at_path(
        &mut self,
        resource: &mut JsonValue,
        segments: &[PathSegment],
        value: JsonValue,
    ) -> Result<(), ExportError> {
        if segments.is_empty() {
            return Err(ExportError::InvalidPath {
                path: "<empty>".to_string(),
                resource: "".to_string(),
            });
        }

        // Start at the root
        let mut current_value = resource;

        // Process each segment
        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;

            current_value = match segment {
                PathSegment::Field(field) => {
                    if is_last {
                        // Set the final value
                        if let JsonValue::Object(obj) = current_value {
                            // Check if this field should be an array
                            let final_value = if self.is_array_field(field) && !value.is_array() {
                                // Wrap scalar value in an array
                                trace!("Wrapping value in array for field '{}'", field);
                                JsonValue::Array(vec![value.clone()])
                            } else {
                                value.clone()
                            };

                            // If the field already exists and both are objects, merge them
                            if let Some(existing) = obj.get_mut(field) {
                                if existing.is_object() && final_value.is_object() {
                                    Self::merge_objects(existing, &final_value);
                                } else {
                                    *existing = final_value;
                                }
                            } else {
                                obj.insert(field.clone(), final_value);
                            }
                            return Ok(());
                        } else {
                            return Err(ExportError::InvalidPath {
                                path: field.clone(),
                                resource: "".to_string(),
                            });
                        }
                    } else {
                        // Navigate or create intermediate structure
                        if let JsonValue::Object(obj) = current_value {
                            if !obj.contains_key(field) {
                                // Determine if we should create an array or object
                                // If the next segment is a Field (not ArrayAccess), this might be
                                // FSH shorthand for accessing array element properties without index
                                let next_is_field = i + 1 < segments.len()
                                    && matches!(segments[i + 1], PathSegment::Field(_));

                                if next_is_field {
                                    // Create an array with one empty object - FSH shorthand for identifier.use
                                    // means identifier[0].use when identifier is an array field
                                    let arr = vec![JsonValue::Object(Map::new())];
                                    obj.insert(field.clone(), JsonValue::Array(arr));
                                } else {
                                    // Create a regular object
                                    obj.insert(field.clone(), JsonValue::Object(Map::new()));
                                }
                            }

                            // Navigate into the field
                            let field_value = obj.get_mut(field).unwrap();

                            // If it's an array and next is a field, navigate into first element
                            if field_value.is_array() {
                                let arr = field_value.as_array_mut().unwrap();
                                if arr.is_empty() {
                                    arr.push(JsonValue::Object(Map::new()));
                                }
                                // Get current index for this field, default to 0
                                let idx = *self.current_indices.get(field).unwrap_or(&0);
                                let arr_len = arr.len();
                                &mut arr[idx.min(arr_len - 1)]
                            } else {
                                field_value
                            }
                        } else {
                            return Err(ExportError::InvalidPath {
                                path: field.clone(),
                                resource: "".to_string(),
                            });
                        }
                    }
                }
                PathSegment::ArrayAccess {
                    field,
                    index,
                    slice_name,
                } => {
                    // Ensure parent is an object
                    if let JsonValue::Object(obj) = current_value {
                        // Ensure array exists
                        if !obj.contains_key(field) {
                            obj.insert(field.clone(), JsonValue::Array(Vec::new()));
                        }

                        let array_value = obj.get_mut(field).unwrap();

                        // Check if it's an array
                        if !array_value.is_array() {
                            return Err(ExportError::TypeMismatch {
                                expected: "array".to_string(),
                                actual: "object".to_string(),
                            });
                        }

                        let arr = array_value.as_array_mut().unwrap();

                        // Determine actual index based on slice name or numeric index
                        let actual_index = if let Some(slice) = slice_name {
                            // Find or create element with matching slice name
                            self.find_or_create_slice(arr, slice, field).await?
                        } else {
                            match index {
                                ArrayIndex::Numeric(n) => *n,
                                ArrayIndex::Append => arr.len(),
                                ArrayIndex::Current => {
                                    *self.current_indices.get(field).unwrap_or(&0)
                                }
                            }
                        };

                        // Ensure array is large enough
                        while arr.len() <= actual_index {
                            arr.push(JsonValue::Object(Map::new()));
                        }

                        // Update current index tracker
                        self.current_indices.insert(field.clone(), actual_index);

                        if is_last {
                            // Set the value at this array index
                            // If existing value is an object and new value is an object, merge
                            if arr[actual_index].is_object() && value.is_object() {
                                Self::merge_objects(&mut arr[actual_index], &value);
                            } else {
                                arr[actual_index] = value.clone();
                            }
                            return Ok(());
                        } else {
                            // Navigate into array element
                            &mut arr[actual_index]
                        }
                    } else {
                        return Err(ExportError::InvalidPath {
                            path: field.clone(),
                            resource: "".to_string(),
                        });
                    }
                }
            };
        }

        Ok(())
    }

    /// Find or create an array element with a specific slice name
    ///
    /// For extensions, this matches by the `url` field and resolves slice names to canonical URLs.
    /// For other slices, this uses the `_sliceName` internal field.
    async fn find_or_create_slice(
        &self,
        arr: &mut Vec<JsonValue>,
        slice_name: &str,
        field: &str,
    ) -> Result<usize, ExportError> {
        // For extensions, match by URL and resolve slice names to canonical URLs
        if field == "extension" || field == "modifierExtension" {
            // Resolve slice name to canonical URL
            let extension_url = self.resolve_extension_url(slice_name).await;

            trace!(
                "Resolving extension slice '{}' -> canonical URL '{}'",
                slice_name, extension_url
            );

            // Try to find existing extension with this URL
            for (idx, elem) in arr.iter().enumerate() {
                if let Some(url) = elem.get("url").and_then(|u| u.as_str())
                    && url == extension_url
                {
                    return Ok(idx);
                }
            }
            // Not found, create new element at end with canonical URL
            let idx = arr.len();
            arr.push(serde_json::json!({
                "url": extension_url
            }));
            Ok(idx)
        } else {
            // For other slices, match by _sliceName
            for (idx, elem) in arr.iter().enumerate() {
                if let Some(name) = elem.get("_sliceName").and_then(|n| n.as_str())
                    && name == slice_name
                {
                    return Ok(idx);
                }
            }
            // Not found, create new element at end
            let idx = arr.len();
            arr.push(serde_json::json!({
                "_sliceName": slice_name
            }));
            Ok(idx)
        }
    }

    /// Resolve extension slice name to canonical URL
    ///
    /// This method attempts to resolve extension slice names in the following order:
    /// 1. If it's already a URL (contains "://"), use it as-is
    /// 2. If it's an alias, resolve to canonical URL via alias table
    /// 3. Check the current profile's slice map extracted from its StructureDefinition
    /// 4. Try to find extension definition in tank/canonical
    /// 5. Fall back to using slice name as-is (for backwards compatibility)
    async fn resolve_extension_url(&self, slice_name: &str) -> String {
        // If it's already a full URL, use as-is
        if slice_name.contains("://") {
            return slice_name.to_string();
        }

        // Try alias resolution first
        if let Some(fishing_ctx) = &self.fishing_context {
            if let Some(canonical_url) = fishing_ctx.resolve_alias(slice_name) {
                debug!(
                    "Resolved extension alias '{}' -> '{}'",
                    slice_name, canonical_url
                );
                return canonical_url;
            }

            // Try current profile slice map extracted from StructureDefinition
            if let Some(url) = self.current_extension_urls.get(slice_name) {
                debug!(
                    "Resolved extension slice '{}' using profile slice map -> '{}'",
                    slice_name, url
                );
                return url.clone();
            }

            // Try multiple naming patterns for the extension (id/name variations)
            let candidates = vec![
                slice_name.to_string(),
                format!("us-core-{}", slice_name),
                format!("uscoreext-{}", slice_name),
                Self::pascal_case(slice_name),
                Self::kebab_case(slice_name),
            ];

            for candidate in &candidates {
                trace!("Trying extension name: '{}'", candidate);

                // Try to find extension definition in tank
                if let Some(metadata) = fishing_ctx
                    .fish_metadata(candidate, &[crate::semantic::ResourceType::Extension])
                    .await
                {
                    debug!(
                        "Resolved extension '{}' (tried '{}') from tank -> '{}'",
                        slice_name, candidate, metadata.url
                    );
                    return metadata.url;
                }

                // Try to find extension in canonical packages
                match fishing_ctx.fish_structure_definition(candidate).await {
                    Ok(Some(sd)) => {
                        debug!(
                            "Resolved extension '{}' (tried '{}') from canonical -> '{}'",
                            slice_name, candidate, sd.url
                        );
                        return sd.url;
                    }
                    Ok(None) | Err(_) => {
                        // Not found with this candidate, try next
                    }
                }
            }
        }

        // Fallback: use slice name as-is
        debug!("Could not resolve extension '{}', using as-is", slice_name);
        slice_name.to_string()
    }

    /// Build a map of extension slice names to canonical URLs from a StructureDefinition
    ///
    /// This inspects the differential first (preferred) and falls back to snapshot.
    /// It looks for `extension:<slice>.url` elements with fixedUri/fixedCanonical values,
    /// or extension slices with type.profile pointing at an extension definition.
    fn build_extension_url_map(sd: &StructureDefinition) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let elements = sd
            .differential
            .as_ref()
            .map(|d| d.element.as_slice())
            .or_else(|| sd.snapshot.as_ref().map(|s| s.element.as_slice()))
            .unwrap_or(&[]);

        for element in elements {
            if let Some(slice_name) = Self::extract_slice_name(element) {
                if element.path.ends_with(".url") {
                    if let Some(url) = Self::extract_fixed_uri(element) {
                        map.entry(slice_name.clone()).or_insert(url);
                        continue;
                    }
                }

                if let Some(url) = Self::extract_type_profile(element) {
                    map.entry(slice_name.clone()).or_insert(url);
                }
            }
        }

        map
    }

    /// Extract slice name from an ElementDefinition using slice_name or path (extension:<slice>)
    fn extract_slice_name(element: &ElementDefinition) -> Option<String> {
        if let Some(name) = element.slice_name.clone() {
            return Some(name);
        }

        element
            .path
            .split('.')
            .find_map(|segment| segment.strip_prefix("extension:"))
            .map(|s| s.to_string())
    }

    /// Extract fixed URI/Canonical from an ElementDefinition
    fn extract_fixed_uri(element: &ElementDefinition) -> Option<String> {
        element.fixed.as_ref().and_then(|fixed_map| {
            fixed_map
                .get("fixedUri")
                .or_else(|| fixed_map.get("fixedCanonical"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    }

    /// Extract extension profile URL from ElementDefinition.type.profile
    fn extract_type_profile(element: &ElementDefinition) -> Option<String> {
        element.type_.as_ref().and_then(|types| {
            types
                .first()
                .and_then(|t| t.profile.as_ref())
                .and_then(|profiles| profiles.first())
                .cloned()
        })
    }

    /// Convert a string to PascalCase (simple heuristic)
    fn pascal_case(input: &str) -> String {
        input
            .split(|c: char| c == '-' || c == '_' || c == ' ')
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>()
    }

    /// Convert a string to kebab-case (simple heuristic)
    fn kebab_case(input: &str) -> String {
        let mut result = String::new();
        for (i, ch) in input.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('-');
                }
                result.push(ch.to_ascii_lowercase());
            } else if ch == '_' || ch == ' ' {
                result.push('-');
            } else {
                result.push(ch);
            }
        }
        result
    }

    /// Load a StructureDefinition by URL or name using the fishing context
    async fn load_structure_definition(
        &self,
        fishing_ctx: &Arc<FishingContext>,
        instance_of: &str,
        canonical_url: Option<&str>,
    ) -> Option<StructureDefinition> {
        if let Some(url) = canonical_url {
            if let Ok(Some(sd)) = fishing_ctx.fish_structure_definition(url).await {
                return Some(sd);
            }
        }

        if let Ok(Some(sd)) = fishing_ctx.fish_structure_definition(instance_of).await {
            return Some(sd);
        }

        let kebab = Self::kebab_case(instance_of);
        let url_candidate = format!("{}/StructureDefinition/{}", self.base_url, kebab);
        if let Ok(Some(sd)) = fishing_ctx.fish_structure_definition(&url_candidate).await {
            return Some(sd);
        }

        None
    }

    /// Merge two JSON objects, combining their properties
    ///
    /// This is used when assigning multiple properties to the same object.
    /// For example:
    /// ```fsh
    /// * contact.name.text = "John Doe"
    /// * contact.name.family = "Doe"
    /// ```
    /// Both rules target `contact.name`, so we merge the values.
    fn merge_objects(target: &mut JsonValue, source: &JsonValue) {
        if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
            for (key, value) in source_obj {
                if let Some(existing) = target_obj.get_mut(key) {
                    if existing.is_object() && value.is_object() {
                        Self::merge_objects(existing, value);
                    } else if existing.is_array() && value.is_array() {
                        // Merge arrays by appending unique elements
                        if let (Some(existing_arr), Some(source_arr)) =
                            (existing.as_array_mut(), value.as_array())
                        {
                            for item in source_arr {
                                if !existing_arr.contains(item) {
                                    existing_arr.push(item.clone());
                                }
                            }
                        }
                    } else {
                        // Replace with new value
                        *existing = value.clone();
                    }
                } else {
                    target_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }

    /// Convert a FSH value string to appropriate JSON value with path context
    ///
    /// This method preserves string types for known FHIR string fields even if the value
    /// looks like a number (e.g., postalCode, identifier.value).
    ///
    /// Handles:
    /// - Simple types: strings, numbers, booleans, codes
    /// - Complex types: CodeableConcept, Quantity, Reference, Ratio
    /// - FHIR-specific patterns
    async fn convert_value_with_path(
        &self,
        value_str: &str,
        path: &str,
    ) -> Result<JsonValue, ExportError> {
        // Check if this path should always be a string
        if self.is_string_field_path(path) {
            let trimmed = value_str.trim();
            // If it's a quoted string, remove quotes
            if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            {
                return Ok(JsonValue::String(trimmed[1..trimmed.len() - 1].to_string()));
            }
            // Otherwise, keep as-is (even if it looks like a number)
            return Ok(JsonValue::String(trimmed.to_string()));
        }

        // Use standard conversion for other fields
        self.convert_value(value_str).await
    }

    /// Check if a path represents a field that should always be a string in FHIR
    ///
    /// This includes fields like postalCode, identifier.value, id, etc. that are
    /// defined as string types in the FHIR spec but often contain numeric-looking values.
    fn is_string_field_path(&self, path: &str) -> bool {
        // Extract the final field name from the path (after last dot)
        let field_name = path.rsplit('.').next().unwrap_or(path);

        // Check for known FHIR string fields that often contain numeric values
        match field_name {
            // Address fields
            "postalCode" | "postal" => true,
            // Identifier fields
            "value" if path.contains("identifier") => true,
            // NPI, SSN, etc.
            "system" if path.contains("identifier") => true,
            // Phone numbers
            "value" if path.contains("telecom") => true,
            // ID fields
            "id" => true,
            // Various other string fields that might look numeric
            "version" | "reference" | "display" => true,
            _ => false,
        }
    }

    /// Check if a field name represents an array field in FHIR
    ///
    /// This includes common FHIR array fields like address.line, name.given,
    /// identifier, telecom, etc.
    fn is_array_field(&self, field_name: &str) -> bool {
        // Check for known FHIR array fields
        matches!(
            field_name,
            // Common array fields across many resources
            "identifier" | "telecom" | "address" | "contact" | "communication" |
            "contained" | "extension" | "modifierExtension" |
            // Name fields
            "name" | "given" | "prefix" | "suffix" |
            // Address fields
            "line" |
            // Coding/CodeableConcept
            "coding" |
            // Observation
            "category" | "performer" | "interpretation" | "note" | "referenceRange" | "component" |
            // Patient
            "photo" | "link" |
            // Practitioner
            "qualification" |
            // Organization
            "alias" | "endpoint" |
            // Condition
            "bodySite" | "stage" | "evidence" |
            // Procedure
            "complication" | "followUp" | "focalDevice" | "usedReference" | "usedCode" |
            // MedicationRequest
            "dosageInstruction" | "detectedIssue" | "eventHistory" |
            // DiagnosticReport
            "result" | "imagingStudy" | "media" |
            // Encounter
            "statusHistory" | "classHistory" | "participant" | "diagnosis" | "account" | "hospitalization" | "location" |
            // Various
            "instantiatesCanonical" | "instantiatesUri" | "basedOn" | "partOf" | "reasonCode" | "reasonReference"
        )
    }

    /// Convert a FSH value string to appropriate JSON value
    ///
    /// Handles:
    /// - Simple types: strings, numbers, booleans, codes
    /// - Complex types: CodeableConcept, Quantity, Reference, Ratio
    /// - FHIR-specific patterns
    async fn convert_value(&self, value_str: &str) -> Result<JsonValue, ExportError> {
        let trimmed = value_str.trim();

        // String literal (remove quotes)
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            return Ok(JsonValue::String(trimmed[1..trimmed.len() - 1].to_string()));
        }

        // Boolean
        if trimmed == "true" {
            return Ok(JsonValue::Bool(true));
        }
        if trimmed == "false" {
            return Ok(JsonValue::Bool(false));
        }

        // Code with system (CodeableConcept pattern): system#code "display"
        // Example: http://loinc.org#LA6576-8 "Excellent"
        if trimmed.contains('#') && !trimmed.starts_with('#') {
            return self.parse_codeable_concept(trimmed).await;
        }

        // Code (starts with #)
        if let Some(code) = trimmed.strip_prefix('#') {
            return Ok(JsonValue::String(code.to_string()));
        }

        // Quantity pattern: 70 'kg'
        if trimmed.contains('\'') && trimmed.chars().next().is_some_and(|c| c.is_numeric()) {
            return self.parse_quantity(trimmed);
        }

        // Reference pattern: Reference(Patient/example)
        if trimmed.starts_with("Reference(") && trimmed.ends_with(')') {
            return self.parse_reference(trimmed).await;
        }

        // Number (integer)
        if let Ok(num) = trimmed.parse::<i64>() {
            return Ok(JsonValue::Number(num.into()));
        }

        // Number (float)
        if let Ok(num) = trimmed.parse::<f64>()
            && let Some(json_num) = serde_json::Number::from_f64(num)
        {
            return Ok(JsonValue::Number(json_num));
        }

        // Check if it's an instance reference (identifier without quotes)
        // Instance references are plain identifiers that match registered instances
        if self.is_instance_reference(trimmed)
            && let Some(instance_json) = self.get_instance(trimmed)
        {
            debug!("Resolved instance reference: {}", trimmed);
            return Ok(instance_json.clone());
        }

        // Default to string
        Ok(JsonValue::String(trimmed.to_string()))
    }

    /// Check if a value looks like an instance reference
    /// Instance references are plain identifiers (alphanumeric + hyphen/underscore)
    fn is_instance_reference(&self, value: &str) -> bool {
        // Must not be empty
        if value.is_empty() {
            return false;
        }

        // Must start with a letter
        if !value.chars().next().unwrap().is_alphabetic() {
            return false;
        }

        // Must contain only valid identifier characters
        value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    /// Parse CodeableConcept from FSH notation
    /// Format: system#code "display" or ALIAS#code "display"
    async fn parse_codeable_concept(&self, value: &str) -> Result<JsonValue, ExportError> {
        // Split into system#code and display
        let parts: Vec<&str> = value.splitn(2, '#').collect();
        if parts.len() != 2 {
            return Ok(JsonValue::String(value.to_string()));
        }

        let system_or_alias = parts[0].trim();
        let code_and_display = parts[1];

        // Resolve system alias to canonical URL
        let system = if let Some(fishing_ctx) = &self.fishing_context {
            if let Some(canonical_url) = fishing_ctx.resolve_alias(system_or_alias) {
                debug!(
                    "Resolved CodeSystem alias '{}' -> '{}'",
                    system_or_alias, canonical_url
                );
                canonical_url
            } else {
                // Not an alias, use as-is
                system_or_alias.to_string()
            }
        } else {
            // No fishing context, use as-is
            system_or_alias.to_string()
        };

        // Extract code and display
        let (code, display) = if let Some(space_idx) = code_and_display.find(' ') {
            let code = code_and_display[..space_idx].trim();
            let display_part = code_and_display[space_idx..].trim();
            let display = if display_part.starts_with('"') && display_part.ends_with('"') {
                &display_part[1..display_part.len() - 1]
            } else {
                display_part
            };
            (code, Some(display))
        } else {
            (code_and_display.trim(), None)
        };

        // Build CodeableConcept
        let mut codeable_concept = serde_json::json!({
            "coding": [{
                "system": system,
                "code": code
            }]
        });

        if let Some(display_text) = display {
            codeable_concept["coding"][0]["display"] = JsonValue::String(display_text.to_string());
        }

        Ok(codeable_concept)
    }

    /// Parse Quantity from FSH notation
    /// Format: 70 'kg' or 5.5 'cm'
    fn parse_quantity(&self, value: &str) -> Result<JsonValue, ExportError> {
        // Extract value and unit
        let parts: Vec<&str> = value.splitn(2, '\'').collect();
        if parts.len() < 2 {
            return Ok(JsonValue::String(value.to_string()));
        }

        let value_str = parts[0].trim();
        let unit_with_quote = parts[1];
        let unit = unit_with_quote.trim_end_matches('\'').trim();

        // Parse numeric value
        let numeric_value = if let Ok(num) = value_str.parse::<i64>() {
            serde_json::json!(num)
        } else if let Ok(num) = value_str.parse::<f64>() {
            serde_json::json!(num)
        } else {
            return Ok(JsonValue::String(value.to_string()));
        };

        // Build Quantity
        let mut quantity = serde_json::json!({
            "value": numeric_value,
            "unit": unit
        });

        // Add system and code for UCUM units
        if unit == "kg" || unit == "g" || unit == "cm" || unit == "m" || unit == "s" {
            quantity["system"] = JsonValue::String("http://unitsofmeasure.org".to_string());
            quantity["code"] = JsonValue::String(unit.to_string());
        }

        Ok(quantity)
    }

    /// Parse Reference from FSH notation
    /// Format: Reference(Patient/example) or Reference(resourceId)
    async fn parse_reference(&self, value: &str) -> Result<JsonValue, ExportError> {
        // Extract reference from Reference(...)
        let ref_value = value
            .strip_prefix("Reference(")
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(value);

        // Validate the reference if fishing context is available
        if let Err(e) = self.validate_reference(ref_value).await {
            warn!("Reference validation failed for '{}': {}", ref_value, e);
            // Note: We don't fail export on invalid references, just warn
            // This matches SUSHI behavior for better user experience
        }

        // Build Reference
        let reference = serde_json::json!({
            "reference": ref_value
        });

        Ok(reference)
    }

    /// Validate that a reference target exists
    ///
    /// Checks if the referenced resource exists in:
    /// 1. Local instance registry (inline instances)
    /// 2. FSH tank (parsed FSH resources)
    /// 3. Canonical packages (external FHIR resources)
    ///
    /// # Arguments
    ///
    /// * `reference` - The reference string (e.g., "Patient/example" or "my-patient-instance")
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Reference is valid
    /// * `Err(ExportError)` - Reference target not found or validation failed
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use maki_core::export::InstanceExporter;
    ///
    /// # async fn example(exporter: &InstanceExporter) -> Result<(), Box<dyn std::error::Error>> {
    /// // Validate a FHIR reference
    /// exporter.validate_reference("Patient/example").await?;
    ///
    /// // Validate an instance reference
    /// exporter.validate_reference("my-patient-instance").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_reference(&self, reference: &str) -> Result<(), ExportError> {
        trace!("Validating reference: {}", reference);

        // Check if it's an inline instance reference (local registry)
        if self.instance_registry.contains_key(reference) {
            debug!("Reference '{}' resolved to inline instance", reference);
            return Ok(());
        }

        // If fishing context is available, check tank and canonical
        if let Some(fishing_ctx) = &self.fishing_context {
            // Try to find the resource in the tank (any resource type)
            if fishing_ctx.fish_metadata(reference, &[]).await.is_some() {
                debug!("Reference '{}' resolved to FSH resource in tank", reference);
                return Ok(());
            }

            // For FHIR-style references like "Patient/example", we can't validate without async
            // but we trust that they're intended references to external resources
            if reference.contains('/') {
                debug!(
                    "Reference '{}' appears to be FHIR-style, trusting it's valid",
                    reference
                );
                return Ok(());
            }
        }

        // Without fishing context, we can only validate inline instances
        // For FHIR-style references, we'll be lenient and assume they're valid
        // This matches SUSHI's behavior where external references are trusted
        if reference.contains('/') {
            debug!(
                "Reference '{}' appears to be FHIR-style (no fishing context), assuming valid",
                reference
            );
            return Ok(());
        }

        // If we get here, we couldn't validate the reference
        // Return error but caller can decide to warn instead of failing
        Err(ExportError::InvalidReference {
            reference: reference.to_string(),
            reason: "Reference target not found in instances, tank, or canonical packages"
                .to_string(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_exporter() -> InstanceExporter {
        InstanceExporter {
            session: Arc::new(crate::canonical::DefinitionSession::for_testing()),
            fishing_context: None,
            base_url: "http://example.org/fhir".to_string(),
            current_indices: HashMap::new(),
            instance_registry: HashMap::new(),
            current_profile_name: None,
            current_profile_url: None,
            current_extension_urls: HashMap::new(),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_simple_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("name.family").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Field("name".to_string()));
        assert_eq!(segments[1], PathSegment::Field("family".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_array_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("name.given[0]").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Field("name".to_string()));
        assert_eq!(
            segments[1],
            PathSegment::ArrayAccess {
                field: "given".to_string(),
                index: ArrayIndex::Numeric(0),
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_append_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("name.given[+]").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[1],
            PathSegment::ArrayAccess {
                field: "given".to_string(),
                index: ArrayIndex::Append,
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_current_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("telecom[=].value").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            PathSegment::ArrayAccess {
                field: "telecom".to_string(),
                index: ArrayIndex::Current,
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_nested_array_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("address[0].line[+]").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            PathSegment::ArrayAccess {
                field: "address".to_string(),
                index: ArrayIndex::Numeric(0),
                slice_name: None
            }
        );
        assert_eq!(
            segments[1],
            PathSegment::ArrayAccess {
                field: "line".to_string(),
                index: ArrayIndex::Append,
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_string_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("\"Hello World\"").await.unwrap();
        assert_eq!(value, JsonValue::String("Hello World".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_boolean_value() {
        let exporter = create_test_exporter();
        assert_eq!(
            exporter.convert_value("true").await.unwrap(),
            JsonValue::Bool(true)
        );
        assert_eq!(
            exporter.convert_value("false").await.unwrap(),
            JsonValue::Bool(false)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_integer_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("42").await.unwrap();
        assert_eq!(value, JsonValue::Number(42.into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_code_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("#male").await.unwrap();
        assert_eq!(value, JsonValue::String("male".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_simple_value() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        let segments = exporter.parse_path("birthDate").unwrap();
        let value = JsonValue::String("1970-01-01".to_string());

        exporter
            .set_value_at_path(&mut resource, &segments, value)
            .await
            .unwrap();

        assert_eq!(resource["birthDate"], "1970-01-01");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_array_value_numeric() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        let segments = exporter.parse_path("name.given[0]").unwrap();
        let value = JsonValue::String("John".to_string());

        exporter
            .set_value_at_path(&mut resource, &segments, value)
            .await
            .unwrap();

        assert_eq!(resource["name"]["given"][0], "John");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_array_value_append() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Add first element
        let segments = exporter.parse_path("name.given[+]").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("John".to_string()),
            )
            .await
            .unwrap();

        // Add second element
        let segments = exporter.parse_path("name.given[+]").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("Jacob".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(resource["name"]["given"][0], "John");
        assert_eq!(resource["name"]["given"][1], "Jacob");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_nested_array_access() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Set address[0].line[0]
        let segments = exporter.parse_path("address[0].line[0]").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("123 Main St".to_string()),
            )
            .await
            .unwrap();

        // Set address[0].city
        let segments = exporter.parse_path("address[0].city").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("Boston".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(resource["address"][0]["line"][0], "123 Main St");
        assert_eq!(resource["address"][0]["city"], "Boston");
    }

    // ===== Reference Validation Tests =====

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_inline_instance() {
        let mut exporter = create_test_exporter();

        exporter.register_instance(
            "my-patient".to_string(),
            serde_json::json!({
                "resourceType": "Patient",
                "id": "my-patient"
            }),
        );

        assert!(exporter.validate_reference("my-patient").await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_not_found() {
        let exporter = create_test_exporter();

        let result = exporter.validate_reference("nonexistent").await;
        assert!(result.is_err());
        if let Err(ExportError::InvalidReference { reference, reason }) = result {
            assert_eq!(reference, "nonexistent");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected InvalidReference error");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_with_fishing_context() {
        use crate::Location;
        use crate::semantic::{FhirResource, FshTank, Package, ResourceType};
        use tokio::sync::RwLock;

        let mut exporter = create_test_exporter();

        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(crate::canonical::DefinitionSession::for_testing());

        {
            let mut t = tank.write().await;
            t.add_resource(FhirResource {
                resource_type: ResourceType::Profile,
                id: "PatientProfile".to_string(),
                name: Some("PatientProfile".to_string()),
                title: None,
                description: None,
                parent: Some("Patient".to_string()),
                elements: Vec::new(),
                location: Location::default(),
                metadata: crate::semantic::ResourceMetadata::default(),
            });
        }

        let fishing_ctx = Arc::new(FishingContext::new(session, tank, package));
        exporter.fishing_context = Some(fishing_ctx);

        assert!(exporter.validate_reference("PatientProfile").await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_fhir_style() {
        let exporter = create_test_exporter();

        assert!(exporter.validate_reference("Patient/example").await.is_ok());
        assert!(
            exporter
                .validate_reference("Observation/vital-signs")
                .await
                .is_ok()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_reference_validates() {
        let mut exporter = create_test_exporter();

        exporter.register_instance(
            "my-patient".to_string(),
            serde_json::json!({
                "resourceType": "Patient",
                "id": "my-patient"
            }),
        );

        let result = exporter.parse_reference("Reference(my-patient)").await;
        assert!(result.is_ok());
        let reference = result.unwrap();
        assert_eq!(reference["reference"], "my-patient");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_reference_warns_on_invalid() {
        let exporter = create_test_exporter();

        let result = exporter.parse_reference("Reference(nonexistent)").await;
        assert!(result.is_ok());
        let reference = result.unwrap();
        assert_eq!(reference["reference"], "nonexistent");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_inline_instance_resolution() {
        let mut exporter = create_test_exporter();

        let patient_json = serde_json::json!({
            "resourceType": "Patient",
            "id": "example-patient",
            "name": [{
                "family": "Doe",
                "given": ["John"]
            }]
        });

        exporter.register_instance("example-patient".to_string(), patient_json.clone());

        let value = exporter.convert_value("example-patient").await.unwrap();
        assert_eq!(value, patient_json);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_is_instance_reference() {
        let exporter = create_test_exporter();

        // Valid instance reference patterns
        assert!(exporter.is_instance_reference("my-patient"));
        assert!(exporter.is_instance_reference("Patient123"));
        assert!(exporter.is_instance_reference("example_instance"));

        // Invalid patterns
        assert!(!exporter.is_instance_reference("")); // empty
        assert!(!exporter.is_instance_reference("123patient")); // starts with number
        assert!(!exporter.is_instance_reference("patient.name")); // contains dot
        assert!(!exporter.is_instance_reference("\"patient\"")); // quoted
        assert!(!exporter.is_instance_reference("#code")); // code
    }

    // ===== Slice Name Tests =====

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_slice_name_path() {
        let exporter = create_test_exporter();
        let segments = exporter
            .parse_path("extension[myExtension].valueString")
            .unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            PathSegment::ArrayAccess {
                field: "extension".to_string(),
                index: ArrayIndex::Numeric(0),
                slice_name: Some("myExtension".to_string())
            }
        );
        assert_eq!(segments[1], PathSegment::Field("valueString".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_extension_by_slice_name() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Set extension URL
        let segments = exporter
            .parse_path("extension[http://example.org/ext].valueString")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("test value".to_string()),
            )
            .await
            .unwrap();

        // Verify the extension was created with URL
        assert!(resource["extension"].is_array());
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0]["url"], "http://example.org/ext");
        assert_eq!(extensions[0]["valueString"], "test value");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_extensions_by_slice_name() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Add first extension
        let segments = exporter
            .parse_path("extension[http://example.org/ext1].valueString")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("value1".to_string()),
            )
            .await
            .unwrap();

        // Add second extension
        let segments = exporter
            .parse_path("extension[http://example.org/ext2].valueInteger")
            .unwrap();
        exporter
            .set_value_at_path(&mut resource, &segments, JsonValue::Number(42.into()))
            .await
            .unwrap();

        // Verify both extensions exist
        assert!(resource["extension"].is_array());
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 2);
        assert_eq!(extensions[0]["url"], "http://example.org/ext1");
        assert_eq!(extensions[0]["valueString"], "value1");
        assert_eq!(extensions[1]["url"], "http://example.org/ext2");
        assert_eq!(extensions[1]["valueInteger"], 42);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_existing_extension() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Add extension
        let segments = exporter
            .parse_path("extension[http://example.org/ext].valueString")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("initial".to_string()),
            )
            .await
            .unwrap();

        // Update the same extension with additional property
        let segments = exporter
            .parse_path("extension[http://example.org/ext].id")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("ext-id".to_string()),
            )
            .await
            .unwrap();

        // Verify extension has both properties
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0]["url"], "http://example.org/ext");
        assert_eq!(extensions[0]["valueString"], "initial");
        assert_eq!(extensions[0]["id"], "ext-id");
    }
}
