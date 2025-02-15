//! use the [`hcl-edit`][hcl_edit] crate to remove values from HCL documents

use std::error::Error;

use hcl_edit::{
    structure::{Body, Structure},
    visit_mut::VisitMut,
};

use crate::parser::Field;

struct HclDeleter {
    fields: Vec<Field>,
    next: Option<Field>,
    error: Option<Box<dyn Error>>,
}

impl HclDeleter {
    fn new(fields: Vec<Field>) -> Self {
        HclDeleter {
            fields,
            next: None,
            error: None,
        }
    }

    fn next_field(&mut self) {
        if self.next.is_none() && !self.fields.is_empty() {
            self.next = Some(self.fields.remove(0));
        }
    }
}

impl VisitMut for HclDeleter {
    fn visit_body_mut(&mut self, node: &mut Body) {
        self.next_field();
        // create a clone so that we can later mutate `self.next`
        let next = self.next.clone();
        if let Some(ref next) = next {
            let mut delete_indices = Vec::new();
            for (index, item) in node.iter().enumerate() {
                match item {
                    Structure::Attribute(attr) => {
                        if attr.key.as_str() == next.name {
                            // this attribute matches, save its index
                            delete_indices.push(index);
                        }
                    }
                    Structure::Block(block) => {
                        if block.ident.as_str() == next.name {
                            if next.labels.is_empty() {
                                if self.fields.is_empty() {
                                    // this block matches, save its index
                                    delete_indices.push(index);
                                }
                            } else {
                                for filter_label in &next.labels {
                                    for block_label in &block.labels {
                                        if block_label.as_str() == filter_label {
                                            delete_indices.push(index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // working from the last item, delete items at these indices
            delete_indices.reverse();
            for index in delete_indices {
                node.remove(index);
            }

            // check again for matches, these indicate that there are additional filter segments
            // (because if there was a match above, then the matching item is already gone)
            for block in node.blocks_mut() {
                if block.ident.as_str() == next.name {
                    if next.labels.is_empty() {
                        // traverse to the next field
                        self.next = Some(self.fields.remove(0));
                        // then visit the body
                        self.visit_body_mut(&mut block.body);
                    } else {
                        for filter_label in &next.labels {
                            for block_label in &block.labels {
                                if block_label.as_str() == filter_label {
                                    // traverse to the next field
                                    self.next = Some(self.fields.remove(0));
                                    // then visit the body
                                    self.visit_body_mut(&mut block.body);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// given a vector of [`Field`]s, delete the [`Expression`] value that matches that filter
pub fn delete(fields: Vec<Field>, body: &mut Body) -> Result<(), Box<dyn Error>> {
    let mut visitor = HclDeleter::new(fields);
    visitor.visit_body_mut(body);
    if let Some(err) = visitor.error {
        return Err(err);
    }
    Ok(())
}
