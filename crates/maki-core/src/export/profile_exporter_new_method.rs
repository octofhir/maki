/// Generate differential by comparing current snapshot with original (SUSHI approach)
    ///
    /// This compares the modified snapshot (after rules applied) with the original snapshot
    /// (before rules applied) to determine what changed. Only changed elements appear in differential.
    fn generate_differential_from_snapshot(
        &self,
        original_snapshot: &Option<StructureDefinitionSnapshot>,
        current_snapshot: &Option<StructureDefinitionSnapshot>,
        original_mappings: &Option<Vec<StructureDefinitionMapping>>,
        current_mappings: &Option<Vec<StructureDefinitionMapping>>,
    ) -> StructureDefinitionDifferential {
        let mut differential_elements = Vec::new();

        // Get elements from both snapshots
        let original_elements = original_snapshot
            .as_ref()
            .map(|s| &s.element)
            .unwrap_or(&vec![]);

        let current_elements = current_snapshot
            .as_ref()
            .map(|s| &s.element)
            .unwrap_or(&vec![]);

        // Compare each element in current snapshot with original
        for current_elem in current_elements {
            // Find matching element in original snapshot
            let original_elem = original_elements
                .iter()
                .find(|e| e.id == current_elem.id || e.path == current_elem.path);

            if let Some(orig) = original_elem {
                // Element exists in both - check if it changed
                if self.element_has_diff(orig, current_elem) {
                    // Create differential element with only changed fields
                    let diff_elem = self.create_diff_element(orig, current_elem);
                    differential_elements.push(diff_elem);
                }
            } else {
                // Element is new (e.g., a slice) - include entire element
                differential_elements.push(current_elem.clone());
            }
        }

        debug!(
            "Generated {} differential elements from snapshot comparison",
            differential_elements.len()
        );

        StructureDefinitionDifferential {
            element: differential_elements,
        }
    }

    /// Check if an element has differences between original and current
    fn element_has_diff(&self, original: &ElementDefinition, current: &ElementDefinition) -> bool {
        // Check all fields that can be constrained
        original.min != current.min
            || original.max != current.max
            || original.must_support != current.must_support
            || original.short != current.short
            || original.definition != current.definition
            || original.comment != current.comment
            || original.type_field != current.type_field
            || original.binding != current.binding
            || original.constraint != current.constraint
            || original.mapping != current.mapping
            || original.fixed != current.fixed
            || original.pattern != current.pattern
    }

    /// Create a differential element containing only changed fields
    fn create_diff_element(
        &self,
        original: &ElementDefinition,
        current: &ElementDefinition,
    ) -> ElementDefinition {
        let mut diff_elem = ElementDefinition::new(current.path.clone());

        // Always include id and path
        diff_elem.id = current.id.clone();

        // Include only fields that changed
        if original.min != current.min {
            diff_elem.min = current.min;
        }
        if original.max != current.max {
            diff_elem.max = current.max.clone();
        }
        if original.must_support != current.must_support {
            diff_elem.must_support = current.must_support;
        }
        if original.short != current.short {
            diff_elem.short = current.short.clone();
        }
        if original.definition != current.definition {
            diff_elem.definition = current.definition.clone();
        }
        if original.comment != current.comment {
            diff_elem.comment = current.comment.clone();
        }
        if original.type_field != current.type_field {
            diff_elem.type_field = current.type_field.clone();
        }
        if original.binding != current.binding {
            diff_elem.binding = current.binding.clone();
        }
        if original.constraint != current.constraint {
            diff_elem.constraint = current.constraint.clone();
        }
        if original.mapping != current.mapping {
            diff_elem.mapping = current.mapping.clone();
        }
        if original.fixed != current.fixed {
            diff_elem.fixed = current.fixed.clone();
        }
        if original.pattern != current.pattern {
            diff_elem.pattern = current.pattern.clone();
        }

        diff_elem
    }
