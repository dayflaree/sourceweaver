//! Preview transform, drawing, table, and filtering helpers.

use super::*;

pub(crate) fn build_source_colored_preview(
    inputs: &[MergeInput],
    report: &MergeReport,
) -> PreviewDocument {
    let previews = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let mut preview = preview_document_with_source(
                &input.document,
                Some(index),
                Some(input.label.as_str()),
            );
            let offset = report
                .applied_offsets
                .iter()
                .find(|(label, _)| label == &input.label)
                .map(|(_, offset)| *offset)
                .unwrap_or(sourceweaver_core::Vec3::ZERO);
            translate_preview_document(&mut preview, offset);
            preview
        })
        .collect::<Vec<_>>();
    combine_preview_documents(previews)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PreviewTransform {
    rect: egui::Rect,
    bounds: PreviewBounds,
    view: PreviewView,
    scale: f32,
    center_u: f64,
    center_v: f64,
    pan: egui::Vec2,
    yaw: f32,
    pitch: f32,
}

impl PreviewTransform {
    pub(crate) fn new(
        rect: egui::Rect,
        bounds: PreviewBounds,
        view: PreviewView,
        zoom: f32,
        pan: egui::Vec2,
        yaw: f32,
        pitch: f32,
    ) -> Self {
        let (min_u, min_v, max_u, max_v) = projected_bounds(bounds, view, yaw, pitch);
        let extent_u = (max_u - min_u).abs().max(1.0);
        let extent_v = (max_v - min_v).abs().max(1.0);
        let fit_scale = ((rect.width() * 0.88) as f64 / extent_u)
            .min((rect.height() * 0.88) as f64 / extent_v)
            .clamp(0.001, 128.0) as f32;
        Self {
            rect,
            bounds,
            view,
            scale: fit_scale * zoom,
            center_u: (min_u + max_u) * 0.5,
            center_v: (min_v + max_v) * 0.5,
            pan,
            yaw,
            pitch,
        }
    }

    pub(crate) fn world_to_screen(&self, point: sourceweaver_core::Vec3) -> egui::Pos2 {
        let (u, v) = self.project_vec(point);
        self.uv_to_screen(u, v)
    }

    fn project_vec(&self, point: sourceweaver_core::Vec3) -> (f64, f64) {
        project_vec(point, self.view, self.yaw, self.pitch)
    }

    fn uv_to_screen(&self, u: f64, v: f64) -> egui::Pos2 {
        self.rect.center()
            + self.pan
            + egui::vec2(
                ((u - self.center_u) as f32) * self.scale,
                -((v - self.center_v) as f32) * self.scale,
            )
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_entity_table(
    ui: &mut egui::Ui,
    map_path: &Path,
    records: &[EntityRecord],
    selected_rows: &mut BTreeSet<EntitySelectionKey>,
    search: &mut String,
    role_filter: &mut Option<BrushRole>,
    sort_column: &mut EntitySortColumn,
    sort_ascending: &mut bool,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) {
    let row_keys = records
        .iter()
        .map(|record| entity_selection_key(map_path, record))
        .collect::<Vec<_>>();

    ui.horizontal_wrapped(|ui| {
        ui.label("Search entities:");
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(260.0)
                .hint_text("classname, targetname, category, description, role"),
        );
        if !search.trim().is_empty() && ui.button("Clear search").clicked() {
            search.clear();
        }
        ui.separator();
        ui.label("Role:");
        egui::ComboBox::from_id_salt("entity_role_filter")
            .selected_text(
                role_filter
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "All roles".to_string()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(role_filter, None, "All roles");
                for role in entity_role_filter_options() {
                    ui.selectable_value(role_filter, Some(role.clone()), role.to_string());
                }
            });
    });

    ui.horizontal_wrapped(|ui| {
        ui.label("Sort:");
        egui::ComboBox::from_id_salt("entity_sort_column")
            .selected_text(entity_sort_column_label(*sort_column))
            .show_ui(ui, |ui| {
                for column in [
                    EntitySortColumn::Index,
                    EntitySortColumn::Block,
                    EntitySortColumn::Category,
                    EntitySortColumn::Classname,
                    EntitySortColumn::Targetname,
                    EntitySortColumn::Origin,
                    EntitySortColumn::Solids,
                    EntitySortColumn::Roles,
                ] {
                    ui.selectable_value(sort_column, column, entity_sort_column_label(column));
                }
            });
        ui.checkbox(sort_ascending, "Ascending");
    });

    let mut rows = records
        .iter()
        .zip(row_keys.iter())
        .filter(|(record, _)| {
            entity_matches_filters(record, search, role_filter.as_ref(), fgd_metadata)
        })
        .collect::<Vec<_>>();
    sort_entity_rows(&mut rows, *sort_column, *sort_ascending, fgd_metadata);

    let filtered_keys = rows
        .iter()
        .map(|(_, key)| (*key).clone())
        .collect::<Vec<_>>();
    let current_selected = row_keys
        .iter()
        .filter(|key| selected_rows.contains(key))
        .count();
    let filtered_selected = filtered_keys
        .iter()
        .filter(|key| selected_rows.contains(key))
        .count();

    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "Showing {} of {} world/entity records",
            rows.len(),
            records.len()
        ));
        ui.separator();
        ui.label(format!(
            "{} selected visible, {} selected in this map, {} selected total",
            filtered_selected,
            current_selected,
            selected_rows.len()
        ));
        ui.separator();
        if ui.button("Select visible rows").clicked() {
            selected_rows.extend(filtered_keys.iter().cloned());
        }
        if ui.button("Select all rows").clicked() {
            selected_rows.extend(row_keys.iter().cloned());
        }
        if ui.button("Clear visible").clicked() {
            for key in &filtered_keys {
                selected_rows.remove(key);
            }
        }
        if ui.button("Clear current map").clicked() {
            for key in &row_keys {
                selected_rows.remove(key);
            }
        }
        if ui.button("Clear all selections").clicked() {
            selected_rows.clear();
        }
    });

    ui.weak("Selections are tracked by VMF path, row index, block name, classname, and targetname so later deletion actions can target rows safely.");

    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("entity_table")
            .striped(true)
            .num_columns(12)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Select");
                ui.strong("#");
                ui.strong("Block");
                ui.strong("Category");
                ui.strong("Friendly name");
                ui.strong("Classname");
                ui.strong("Targetname");
                ui.strong("Origin");
                ui.strong("Solids");
                ui.strong("Roles");
                ui.strong("FGD properties");
                ui.strong("Description");
                ui.end_row();

                for (record, key) in rows {
                    let metadata = record.classname.as_deref().map(|classname| {
                        metadata_for_classname_with_overrides(classname, fgd_metadata)
                    });
                    let mut selected = selected_rows.contains(key);
                    if ui.checkbox(&mut selected, "").changed() {
                        if selected {
                            selected_rows.insert(key.clone());
                        } else {
                            selected_rows.remove(key);
                        }
                    }
                    let text_color = if selected {
                        egui::Color32::YELLOW
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(text_color, record.index.to_string());
                    ui.colored_label(text_color, &record.block_name);
                    ui.colored_label(
                        text_color,
                        metadata
                            .as_ref()
                            .map(|metadata| metadata.category.to_string())
                            .unwrap_or_else(|| "world".to_string()),
                    );
                    ui.colored_label(
                        text_color,
                        metadata
                            .as_ref()
                            .map(|metadata| metadata.display_name.as_str())
                            .unwrap_or("World"),
                    );
                    ui.colored_label(text_color, record.classname.as_deref().unwrap_or("-"));
                    ui.colored_label(text_color, record.targetname.as_deref().unwrap_or("-"));
                    ui.colored_label(
                        text_color,
                        record
                            .origin
                            .map(|origin| origin.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    ui.colored_label(text_color, record.solid_count.to_string());
                    ui.colored_label(text_color, format_roles(&record.roles));
                    let property_summary = metadata
                        .as_ref()
                        .map(property_metadata_summary)
                        .unwrap_or_else(|| "0 properties".to_string());
                    let property_response = ui.colored_label(text_color, property_summary);
                    if let Some(metadata) = &metadata {
                        let tooltip = property_metadata_tooltip(metadata);
                        if !tooltip.is_empty() {
                            property_response.on_hover_text(tooltip);
                        }
                    }
                    ui.colored_label(
                        text_color,
                        metadata
                            .as_ref()
                            .and_then(|metadata| metadata.description.as_deref())
                            .unwrap_or("-"),
                    );
                    ui.end_row();
                }
            });
    });
}

pub(crate) fn entity_selection_key(map_path: &Path, record: &EntityRecord) -> EntitySelectionKey {
    EntitySelectionKey {
        map_path: display_path(map_path),
        record_index: record.index,
        block_name: record.block_name.clone(),
        classname: record.classname.clone(),
        targetname: record.targetname.clone(),
    }
}

pub(crate) fn entity_selection_key_for_owner(
    map_path: &Path,
    records: &[EntityRecord],
    owner_index: usize,
) -> Option<EntitySelectionKey> {
    records
        .iter()
        .find(|record| record.index == owner_index)
        .map(|record| entity_selection_key(map_path, record))
}

pub(crate) fn draw_classname_table(
    ui: &mut egui::Ui,
    type_counts: &BTreeMap<String, usize>,
    search: &mut String,
    sort_column: &mut ClassnameSortColumn,
    sort_ascending: &mut bool,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Search classnames:");
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(260.0)
                .hint_text("classname, category, description"),
        );
        if !search.trim().is_empty() && ui.button("Clear search").clicked() {
            search.clear();
        }
        ui.separator();
        ui.label("Sort:");
        egui::ComboBox::from_id_salt("classname_sort_column")
            .selected_text(classname_sort_column_label(*sort_column))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    sort_column,
                    ClassnameSortColumn::Classname,
                    classname_sort_column_label(ClassnameSortColumn::Classname),
                );
                ui.selectable_value(
                    sort_column,
                    ClassnameSortColumn::Count,
                    classname_sort_column_label(ClassnameSortColumn::Count),
                );
            });
        ui.checkbox(sort_ascending, "Ascending");
    });

    let query = search.trim().to_ascii_lowercase();
    let mut rows = type_counts
        .iter()
        .filter(|(classname, _)| classname_matches_metadata_search(classname, &query, fgd_metadata))
        .collect::<Vec<_>>();
    sort_classname_rows(&mut rows, *sort_column, *sort_ascending);

    ui.label(format!(
        "Showing {} of {} detected classnames",
        rows.len(),
        type_counts.len()
    ));
    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("classname_table")
            .striped(true)
            .num_columns(6)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Count");
                ui.strong("Classname");
                ui.strong("Category");
                ui.strong("Friendly name");
                ui.strong("FGD properties");
                ui.strong("Description");
                ui.end_row();
                for (classname, count) in rows {
                    let metadata = metadata_for_classname_with_overrides(classname, fgd_metadata);
                    ui.label(count.to_string());
                    ui.label(classname);
                    ui.label(metadata.category.to_string());
                    ui.label(&metadata.display_name);
                    let property_response = ui.label(property_metadata_summary(&metadata));
                    let tooltip = property_metadata_tooltip(&metadata);
                    if !tooltip.is_empty() {
                        property_response.on_hover_text(tooltip);
                    }
                    ui.label(metadata.description.as_deref().unwrap_or("-"));
                    ui.end_row();
                }
            });
    });
}

pub(crate) fn property_metadata_summary(metadata: &EntityMetadata) -> String {
    match metadata.properties.len() {
        0 => "0 properties".to_string(),
        1 => metadata
            .properties
            .values()
            .next()
            .map(|property| {
                format!(
                    "1 property: {}",
                    property.label.as_deref().unwrap_or(&property.key)
                )
            })
            .unwrap_or_else(|| "1 property".to_string()),
        count => format!("{count} properties"),
    }
}

pub(crate) fn property_metadata_tooltip(metadata: &EntityMetadata) -> String {
    metadata
        .properties
        .values()
        .take(12)
        .map(|property| {
            let mut line = format!(
                "{} ({})",
                property.label.as_deref().unwrap_or(&property.key),
                property.value_type.as_deref().unwrap_or("unknown")
            );
            if let Some(default_value) = &property.default_value {
                line.push_str(&format!(" default={default_value}"));
            }
            if let Some(description) = &property.description {
                line.push_str(&format!(" — {description}"));
            }
            if !property.choices.is_empty() {
                let choices = property
                    .choices
                    .iter()
                    .take(4)
                    .map(|choice| format!("{}={}", choice.value, choice.label))
                    .collect::<Vec<_>>()
                    .join(", ");
                line.push_str(&format!(" [{choices}]"));
            }
            line
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        )
}

pub(crate) fn property_metadata_search_text(metadata: &EntityMetadata) -> String {
    let mut parts = Vec::new();
    for property in metadata.properties.values() {
        parts.push(property.key.clone());
        if let Some(label) = &property.label {
            parts.push(label.clone());
        }
        if let Some(value_type) = &property.value_type {
            parts.push(value_type.clone());
        }
        if let Some(description) = &property.description {
            parts.push(description.clone());
        }
        for choice in &property.choices {
            parts.push(choice.value.clone());
            parts.push(choice.label.clone());
            if let Some(description) = &choice.description {
                parts.push(description.clone());
            }
        }
    }
    parts.join(" ").to_ascii_lowercase()
}

pub(crate) fn classname_matches_metadata_search(
    classname: &str,
    query: &str,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) -> bool {
    if query.is_empty() {
        return true;
    }
    let metadata = metadata_for_classname_with_overrides(classname, fgd_metadata);
    classname.to_ascii_lowercase().contains(query)
        || metadata.category.to_string().contains(query)
        || metadata.display_name.to_ascii_lowercase().contains(query)
        || metadata
            .description
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(query)
        || property_metadata_search_text(&metadata).contains(query)
}

pub(crate) fn draw_transition_table(ui: &mut egui::Ui, transitions: &[CampaignTransition]) {
    if transitions.is_empty() {
        ui.weak("No trigger_changelevel entities detected in this VMF.");
        return;
    }

    ui.label(format!(
        "{} trigger_changelevel transition(s) detected",
        transitions.len()
    ));
    ui.weak("These target maps and landmarks can guide future stitching and landmark selection.");

    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("transition_table")
            .striped(true)
            .num_columns(6)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Entity #");
                ui.strong("Targetname");
                ui.strong("Target map");
                ui.strong("Landmark");
                ui.strong("Origin");
                ui.strong("Solids");
                ui.end_row();

                for transition in transitions {
                    ui.label(transition.entity_index.to_string());
                    ui.label(transition.targetname.as_deref().unwrap_or("-"));
                    ui.label(transition.target_map.as_deref().unwrap_or("-"));
                    ui.label(transition.landmark.as_deref().unwrap_or("-"));
                    ui.label(
                        transition
                            .origin
                            .map(|origin| origin.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    ui.label(transition.solid_count.to_string());
                    ui.end_row();
                }
            });
    });
}

pub(crate) fn entity_matches_filters(
    record: &EntityRecord,
    search: &str,
    role_filter: Option<&BrushRole>,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) -> bool {
    if role_filter.is_some_and(|role| !record.roles.iter().any(|record_role| record_role == role)) {
        return false;
    }

    let query = search.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }

    let roles = format_roles(&record.roles).to_ascii_lowercase();
    let metadata = record
        .classname
        .as_deref()
        .map(|classname| metadata_for_classname_with_overrides(classname, fgd_metadata));
    record.block_name.to_ascii_lowercase().contains(&query)
        || record
            .classname
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(&query)
        || record
            .targetname
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(&query)
        || metadata
            .as_ref()
            .map(|metadata| {
                metadata.category.to_string().contains(&query)
                    || metadata.display_name.to_ascii_lowercase().contains(&query)
                    || metadata
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .contains(&query)
                    || property_metadata_search_text(metadata).contains(&query)
            })
            .unwrap_or(false)
        || roles.contains(&query)
}

pub(crate) fn sort_entity_rows(
    rows: &mut Vec<(&EntityRecord, &EntitySelectionKey)>,
    column: EntitySortColumn,
    ascending: bool,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) {
    rows.sort_by(|(left, _), (right, _)| {
        let ordering = match column {
            EntitySortColumn::Index => left.index.cmp(&right.index),
            EntitySortColumn::Block => left.block_name.cmp(&right.block_name),
            EntitySortColumn::Category => entity_category_sort_key(left, fgd_metadata)
                .cmp(&entity_category_sort_key(right, fgd_metadata)),
            EntitySortColumn::Classname => left.classname.cmp(&right.classname),
            EntitySortColumn::Targetname => left.targetname.cmp(&right.targetname),
            EntitySortColumn::Origin => left
                .origin
                .map(|origin| origin.to_string())
                .cmp(&right.origin.map(|origin| origin.to_string())),
            EntitySortColumn::Solids => left.solid_count.cmp(&right.solid_count),
            EntitySortColumn::Roles => format_roles(&left.roles).cmp(&format_roles(&right.roles)),
        };
        if ascending {
            ordering.then_with(|| left.index.cmp(&right.index))
        } else {
            ordering
                .reverse()
                .then_with(|| left.index.cmp(&right.index))
        }
    });
}

pub(crate) fn entity_category_sort_key(
    record: &EntityRecord,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) -> String {
    record
        .classname
        .as_deref()
        .map(|classname| {
            metadata_for_classname_with_overrides(classname, fgd_metadata)
                .category
                .to_string()
        })
        .unwrap_or_else(|| "world".to_string())
}

pub(crate) fn sort_classname_rows(
    rows: &mut Vec<(&String, &usize)>,
    column: ClassnameSortColumn,
    ascending: bool,
) {
    rows.sort_by(|(left_name, left_count), (right_name, right_count)| {
        let ordering = match column {
            ClassnameSortColumn::Classname => left_name.cmp(right_name),
            ClassnameSortColumn::Count => left_count.cmp(right_count),
        };
        if ascending {
            ordering.then_with(|| left_name.cmp(right_name))
        } else {
            ordering.reverse().then_with(|| left_name.cmp(right_name))
        }
    });
}

pub(crate) fn entity_role_filter_options() -> Vec<BrushRole> {
    vec![
        BrushRole::Trigger,
        BrushRole::Clip,
        BrushRole::Areaportal,
        BrushRole::Skybox,
        BrushRole::Occluder,
        BrushRole::Hint,
        BrushRole::Skip,
        BrushRole::Nodraw,
        BrushRole::Water,
        BrushRole::WorldBrush,
        BrushRole::BrushEntity,
        BrushRole::Other,
    ]
}

pub(crate) fn entity_sort_column_label(column: EntitySortColumn) -> &'static str {
    match column {
        EntitySortColumn::Index => "Index",
        EntitySortColumn::Block => "Block",
        EntitySortColumn::Category => "Category",
        EntitySortColumn::Classname => "Classname",
        EntitySortColumn::Targetname => "Targetname",
        EntitySortColumn::Origin => "Origin",
        EntitySortColumn::Solids => "Solids",
        EntitySortColumn::Roles => "Roles",
    }
}

pub(crate) fn classname_sort_column_label(column: ClassnameSortColumn) -> &'static str {
    match column {
        ClassnameSortColumn::Classname => "Classname",
        ClassnameSortColumn::Count => "Count",
    }
}

pub(crate) fn draw_preview_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    transform: &PreviewTransform,
) {
    let (min_u, min_v, max_u, max_v) = projected_bounds(
        transform.bounds,
        transform.view,
        transform.yaw,
        transform.pitch,
    );
    let pad_u = ((max_u - min_u).abs() * 0.25).max(512.0);
    let pad_v = ((max_v - min_v).abs() * 0.25).max(512.0);
    let step = nice_grid_step(transform.scale);
    let start_u = ((min_u - pad_u) / step).floor() as i64;
    let end_u = ((max_u + pad_u) / step).ceil() as i64;
    let start_v = ((min_v - pad_v) / step).floor() as i64;
    let end_v = ((max_v + pad_v) / step).ceil() as i64;
    let stroke = egui::Stroke::new(
        1.0_f32,
        egui::Color32::from_rgba_unmultiplied(85, 93, 105, 55),
    );
    let axis_stroke = egui::Stroke::new(
        1.5_f32,
        egui::Color32::from_rgba_unmultiplied(130, 148, 176, 120),
    );

    for i in start_u.max(-500)..=end_u.min(500) {
        let u = i as f64 * step;
        let a = transform.uv_to_screen(u, min_v - pad_v);
        let b = transform.uv_to_screen(u, max_v + pad_v);
        painter.line_segment([a, b], if i == 0 { axis_stroke } else { stroke });
    }
    for i in start_v.max(-500)..=end_v.min(500) {
        let v = i as f64 * step;
        let a = transform.uv_to_screen(min_u - pad_u, v);
        let b = transform.uv_to_screen(max_u + pad_u, v);
        painter.line_segment([a, b], if i == 0 { axis_stroke } else { stroke });
    }

    painter.text(
        rect.left_bottom() + egui::vec2(10.0, -12.0),
        egui::Align2::LEFT_BOTTOM,
        format!("grid {}u", step as i64),
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(150),
    );
}

pub(crate) fn draw_landmark_markers(
    painter: &egui::Painter,
    transform: &PreviewTransform,
    preview: &PreviewDocument,
    selected_landmark: &str,
) {
    let selected_landmark = selected_landmark.trim();
    for landmark in &preview.landmarks {
        let position = transform.world_to_screen(landmark.origin);
        if !transform.rect.contains(position) {
            continue;
        }

        let is_selected = !selected_landmark.is_empty() && landmark.targetname == selected_landmark;
        let base_color = landmark
            .source_index
            .map(source_color)
            .unwrap_or_else(|| egui::Color32::from_rgb(100, 255, 180));
        let color = if is_selected {
            egui::Color32::from_rgb(255, 255, 120)
        } else {
            base_color
        };
        let radius = if is_selected { 9.0 } else { 7.0 };
        let points = [
            position + egui::vec2(0.0, -radius),
            position + egui::vec2(radius, 0.0),
            position + egui::vec2(0.0, radius),
            position + egui::vec2(-radius, 0.0),
        ];

        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            color.gamma_multiply(0.85),
            egui::Stroke::new(
                if is_selected { 2.0_f32 } else { 1.2_f32 },
                egui::Color32::BLACK,
            ),
        ));
        painter.text(
            position + egui::vec2(radius + 5.0, -radius - 2.0),
            egui::Align2::LEFT_CENTER,
            if is_selected {
                format!("★ {}", landmark.targetname)
            } else {
                landmark.targetname.clone()
            },
            egui::FontId::monospace(if is_selected { 11.0 } else { 10.0 }),
            if is_selected {
                egui::Color32::from_rgb(255, 255, 160)
            } else {
                egui::Color32::from_rgb(190, 255, 220)
            },
        );
    }
}

pub(crate) fn draw_offset_arrows(
    painter: &egui::Painter,
    rect: egui::Rect,
    transform: &PreviewTransform,
    summary: &MergedPreviewSummary,
) {
    let non_zero_offsets = summary
        .source_offsets
        .iter()
        .enumerate()
        .filter(|(_, (_, offset))| *offset != sourceweaver_core::Vec3::ZERO)
        .collect::<Vec<_>>();
    if non_zero_offsets.is_empty() {
        return;
    }

    let mut anchor = rect.right_bottom() + egui::vec2(-250.0, -28.0);
    painter.text(
        anchor + egui::vec2(0.0_f32, -16.0_f32),
        egui::Align2::LEFT_CENTER,
        "merge offsets",
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(230),
    );

    for (index, (label, offset)) in non_zero_offsets {
        let color = source_color(index);
        let direction = offset_vector_to_screen_delta(*offset, transform);
        let length = direction.length().clamp(24.0, 72.0);
        let unit = if direction.length() > f32::EPSILON {
            direction / direction.length()
        } else {
            egui::vec2(1.0, 0.0)
        };
        let start = anchor;
        let end = anchor + unit * length;
        painter.line_segment([start, end], egui::Stroke::new(2.0_f32, color));
        let left = end - unit * 8.0_f32 + egui::vec2(-unit.y, unit.x) * 4.0_f32;
        let right = end - unit * 8.0_f32 + egui::vec2(unit.y, -unit.x) * 4.0_f32;
        painter.line_segment([left, end], egui::Stroke::new(2.0_f32, color));
        painter.line_segment([right, end], egui::Stroke::new(2.0_f32, color));
        painter.text(
            start + egui::vec2(82.0_f32, 0.0_f32),
            egui::Align2::LEFT_CENTER,
            format!("{}: {offset}", file_label_for_legend(label)),
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(225),
        );
        anchor.y -= 20.0;
    }
}

pub(crate) fn offset_vector_to_screen_delta(
    offset: sourceweaver_core::Vec3,
    transform: &PreviewTransform,
) -> egui::Vec2 {
    let (u, v) = transform.project_vec(offset);
    egui::vec2(u as f32, -(v as f32))
}

pub(crate) fn draw_preview_solid(
    painter: &egui::Painter,
    transform: &PreviewTransform,
    solid: &PreviewSolid,
    material_index: &MaterialPreviewIndex,
    options: PreviewSolidDrawOptions,
) {
    let rect = preview_solid_screen_rect(solid, transform);
    if !rect.intersects(transform.rect) {
        return;
    }

    let mut role_color = solid_color(solid);
    let mut fill_color = solid.source_index.map(source_color).unwrap_or(role_color);
    if options.removed {
        match options.deletion_mode {
            DeletionPreviewMode::Off | DeletionPreviewMode::HideRemoved => {}
            DeletionPreviewMode::HighlightRemoved => {
                role_color = egui::Color32::from_rgb(255, 40, 40);
                fill_color = egui::Color32::from_rgb(255, 40, 40);
            }
            DeletionPreviewMode::DimRemoved => {
                role_color = role_color.gamma_multiply(0.25);
                fill_color = fill_color.gamma_multiply(0.18);
            }
        }
    }

    let stroke = egui::Stroke::new(
        if options.selected {
            2.5_f32
        } else if options.removed {
            2.0_f32
        } else {
            1.25_f32
        },
        if options.selected {
            egui::Color32::YELLOW
        } else {
            role_color
        },
    );
    let fill = fill_color.gamma_multiply(if options.removed { 0.32 } else { 0.22 });
    let draw_faces =
        should_draw_reconstructed_faces(options.detail_mode, options.solid_count, options.selected);
    let draw_plane_edges =
        should_draw_plane_edges(options.detail_mode, options.solid_count, options.selected);

    let drew_reconstructed_faces = if draw_faces {
        draw_reconstructed_faces(
            painter,
            transform,
            solid,
            material_index,
            options,
            fill,
            stroke,
        )
    } else {
        false
    };

    if !drew_reconstructed_faces {
        painter.rect_filled(rect, 0.0, fill);
        draw_rect_outline(painter, rect, stroke);
    }

    if !draw_plane_edges {
        return;
    }

    for chunk in solid.points.chunks(3) {
        if chunk.len() == 3 {
            let p0 = transform.world_to_screen(chunk[0]);
            let p1 = transform.world_to_screen(chunk[1]);
            let p2 = transform.world_to_screen(chunk[2]);
            painter.line_segment(
                [p0, p1],
                egui::Stroke::new(0.8_f32, role_color.gamma_multiply(0.7)),
            );
            painter.line_segment(
                [p1, p2],
                egui::Stroke::new(0.8_f32, role_color.gamma_multiply(0.7)),
            );
            painter.line_segment(
                [p2, p0],
                egui::Stroke::new(0.8_f32, role_color.gamma_multiply(0.7)),
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PreviewSolidDrawOptions {
    pub(crate) deletion_mode: DeletionPreviewMode,
    pub(crate) removed: bool,
    pub(crate) selected: bool,
    pub(crate) detail_mode: PreviewDetailMode,
    pub(crate) material_preview_enabled: bool,
    pub(crate) solid_count: usize,
}

pub(crate) fn should_draw_reconstructed_faces(
    detail_mode: PreviewDetailMode,
    solid_count: usize,
    selected: bool,
) -> bool {
    selected
        || match detail_mode {
            PreviewDetailMode::Fast => false,
            PreviewDetailMode::Auto => solid_count <= 1_200,
            PreviewDetailMode::Full => true,
        }
}

pub(crate) fn should_draw_plane_edges(
    detail_mode: PreviewDetailMode,
    solid_count: usize,
    selected: bool,
) -> bool {
    selected
        || match detail_mode {
            PreviewDetailMode::Fast => false,
            PreviewDetailMode::Auto => solid_count <= 350,
            PreviewDetailMode::Full => true,
        }
}

pub(crate) fn draw_reconstructed_faces(
    painter: &egui::Painter,
    transform: &PreviewTransform,
    solid: &PreviewSolid,
    material_index: &MaterialPreviewIndex,
    options: PreviewSolidDrawOptions,
    fill: egui::Color32,
    stroke: egui::Stroke,
) -> bool {
    let mut drew_any = false;
    for (face_index, polygon) in solid.face_polygons.iter().enumerate() {
        if polygon.len() < 3 {
            continue;
        }
        let screen_points = polygon
            .iter()
            .map(|point| transform.world_to_screen(*point))
            .collect::<Vec<_>>();
        if projected_polygon_area(&screen_points) < 0.75 {
            continue;
        }
        let rect = screen_points.iter().fold(
            egui::Rect::from_min_size(screen_points[0], egui::Vec2::ZERO),
            |rect, point| rect.union(egui::Rect::from_min_size(*point, egui::Vec2::ZERO)),
        );
        if !rect.intersects(transform.rect) {
            continue;
        }
        let face_fill = if options.material_preview_enabled && !options.removed {
            solid
                .face_materials
                .get(face_index)
                .and_then(|material| material.as_deref())
                .map(|material| material_preview_color(material, material_index))
                .unwrap_or(fill)
                .gamma_multiply(0.62)
        } else {
            fill
        };
        painter.add(egui::Shape::convex_polygon(
            screen_points,
            face_fill,
            stroke,
        ));
        drew_any = true;
    }
    drew_any
}

pub(crate) fn projected_polygon_area(points: &[egui::Pos2]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0_f32;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area.abs() * 0.5
}

pub(crate) fn draw_preview_legend(
    painter: &egui::Painter,
    rect: egui::Rect,
    merged_summary: Option<&MergedPreviewSummary>,
) {
    let items = [
        ("world", egui::Color32::from_rgb(170, 190, 220)),
        ("skybox", egui::Color32::from_rgb(100, 170, 255)),
        ("trigger", egui::Color32::from_rgb(255, 170, 90)),
        ("clip", egui::Color32::from_rgb(230, 95, 95)),
        ("areaportal", egui::Color32::from_rgb(185, 115, 255)),
        ("water", egui::Color32::from_rgb(80, 210, 230)),
        ("entity origin", egui::Color32::from_rgb(255, 232, 128)),
        ("landmark", egui::Color32::from_rgb(100, 255, 180)),
        ("selected landmark", egui::Color32::from_rgb(255, 255, 120)),
    ];
    let mut cursor = rect.left_top() + egui::vec2(12.0, 12.0);
    for (label, color) in items {
        let swatch = egui::Rect::from_min_size(cursor, egui::vec2(10.0, 10.0));
        painter.rect_filled(swatch, 1.0, color);
        painter.text(
            cursor + egui::vec2(16.0, 5.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(220),
        );
        cursor.y += 16.0;
    }

    let Some(summary) = merged_summary else {
        return;
    };
    if summary.source_labels.is_empty() {
        return;
    }
    cursor.y += 8.0;
    painter.text(
        cursor,
        egui::Align2::LEFT_TOP,
        "source maps",
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(235),
    );
    cursor.y += 16.0;
    for (index, label) in summary.source_labels.iter().enumerate() {
        let color = source_color(index);
        let swatch = egui::Rect::from_min_size(cursor, egui::vec2(10.0, 10.0));
        painter.rect_filled(swatch, 1.0, color);
        painter.text(
            cursor + egui::vec2(16.0, 5.0),
            egui::Align2::LEFT_CENTER,
            file_label_for_legend(label),
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(220),
        );
        cursor.y += 16.0;
    }
}

pub(crate) fn source_color(index: usize) -> egui::Color32 {
    const PALETTE: [egui::Color32; 10] = [
        egui::Color32::from_rgb(80, 180, 255),
        egui::Color32::from_rgb(255, 160, 90),
        egui::Color32::from_rgb(145, 220, 115),
        egui::Color32::from_rgb(220, 140, 255),
        egui::Color32::from_rgb(255, 215, 95),
        egui::Color32::from_rgb(95, 220, 210),
        egui::Color32::from_rgb(255, 115, 150),
        egui::Color32::from_rgb(175, 175, 255),
        egui::Color32::from_rgb(210, 180, 120),
        egui::Color32::from_rgb(170, 230, 180),
    ];
    PALETTE[index % PALETTE.len()]
}

pub(crate) fn file_label_for_legend(label: &str) -> String {
    Path::new(label)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(label)
        .to_string()
}

pub(crate) fn draw_axes_label(painter: &egui::Painter, rect: egui::Rect, view: PreviewView) {
    let label = match view {
        PreviewView::Top => "Top view: X / Y",
        PreviewView::Front => "Front view: X / Z",
        PreviewView::Side => "Side view: Y / Z",
        PreviewView::ThreeD => "3D isometric preview",
    };
    painter.text(
        rect.right_top() + egui::vec2(-12.0, 12.0),
        egui::Align2::RIGHT_TOP,
        label,
        egui::FontId::monospace(12.0),
        egui::Color32::from_gray(210),
    );
}

pub(crate) fn draw_rect_outline(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    painter.line_segment([rect.left_top(), rect.right_top()], stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
    painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
}

pub(crate) fn project_vec(
    point: sourceweaver_core::Vec3,
    view: PreviewView,
    yaw: f32,
    pitch: f32,
) -> (f64, f64) {
    match view {
        PreviewView::Top => (point.x, point.y),
        PreviewView::Front => (point.x, point.z),
        PreviewView::Side => (point.y, point.z),
        PreviewView::ThreeD => project_vec_isometric(point, yaw, pitch),
    }
}

pub(crate) fn projected_bounds(
    bounds: PreviewBounds,
    view: PreviewView,
    yaw: f32,
    pitch: f32,
) -> (f64, f64, f64, f64) {
    let corners = [
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ];
    let mut min_u = f64::INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for corner in corners {
        let (u, v) = project_vec(corner, view, yaw, pitch);
        min_u = min_u.min(u);
        min_v = min_v.min(v);
        max_u = max_u.max(u);
        max_v = max_v.max(v);
    }
    (min_u, min_v, max_u, max_v)
}

pub(crate) fn project_vec_isometric(
    point: sourceweaver_core::Vec3,
    yaw: f32,
    pitch: f32,
) -> (f64, f64) {
    let yaw = (yaw as f64).to_radians();
    let pitch = (pitch as f64).to_radians();
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();
    let x = point.x * cos_yaw - point.y * sin_yaw;
    let y = point.x * sin_yaw + point.y * cos_yaw;
    let z = point.z;
    (x, y * cos_pitch - z * sin_pitch)
}

pub(crate) fn preview_hit_owner_index(
    preview: &PreviewDocument,
    transform: &PreviewTransform,
    click_position: egui::Pos2,
    criteria: &DeletionCriteria,
    deletion_mode: DeletionPreviewMode,
) -> Option<usize> {
    let mut nearest_entity = None;
    let mut nearest_distance = f32::MAX;
    for entity in &preview.entities {
        if deletion_mode == DeletionPreviewMode::HideRemoved
            && preview_entity_removed(entity, criteria)
        {
            continue;
        }
        let position = transform.world_to_screen(entity.origin);
        let distance = position.distance(click_position);
        if distance < nearest_distance && distance <= 12.0 {
            nearest_distance = distance;
            nearest_entity = Some(entity.owner_index);
        }
    }
    if nearest_entity.is_some() {
        return nearest_entity;
    }

    for solid in preview.solids.iter().rev() {
        if deletion_mode == DeletionPreviewMode::HideRemoved
            && preview_solid_removed(solid, criteria)
        {
            continue;
        }
        if preview_solid_screen_rect(solid, transform).contains(click_position) {
            return Some(solid.owner_index);
        }
    }
    None
}

pub(crate) fn preview_solid_screen_rect(
    solid: &PreviewSolid,
    transform: &PreviewTransform,
) -> egui::Rect {
    solid_bounds_corners(solid)
        .iter()
        .map(|corner| transform.world_to_screen(*corner))
        .fold(egui::Rect::NOTHING, |rect, point| {
            rect.union(egui::Rect::from_min_size(point, egui::Vec2::ZERO))
        })
}

pub(crate) fn solid_bounds_corners(solid: &PreviewSolid) -> [sourceweaver_core::Vec3; 8] {
    let min = solid.bounds.min;
    let max = solid.bounds.max;
    [
        sourceweaver_core::Vec3::new(min.x, min.y, min.z),
        sourceweaver_core::Vec3::new(min.x, min.y, max.z),
        sourceweaver_core::Vec3::new(min.x, max.y, min.z),
        sourceweaver_core::Vec3::new(min.x, max.y, max.z),
        sourceweaver_core::Vec3::new(max.x, min.y, min.z),
        sourceweaver_core::Vec3::new(max.x, min.y, max.z),
        sourceweaver_core::Vec3::new(max.x, max.y, min.z),
        sourceweaver_core::Vec3::new(max.x, max.y, max.z),
    ]
}

pub(crate) fn deletion_preview_mode_label(mode: DeletionPreviewMode) -> &'static str {
    match mode {
        DeletionPreviewMode::Off => "Off",
        DeletionPreviewMode::HighlightRemoved => "Highlight removed red",
        DeletionPreviewMode::DimRemoved => "Dim removed",
        DeletionPreviewMode::HideRemoved => "Hide removed",
    }
}

pub(crate) fn preview_detail_mode_label(mode: PreviewDetailMode) -> &'static str {
    match mode {
        PreviewDetailMode::Fast => "Fast boxes",
        PreviewDetailMode::Auto => "Auto",
        PreviewDetailMode::Full => "Full faces",
    }
}

pub(crate) fn preview_detail_mode_description(
    mode: PreviewDetailMode,
    solid_count: usize,
) -> &'static str {
    match mode {
        PreviewDetailMode::Fast => "bounding boxes only",
        PreviewDetailMode::Auto if solid_count > 1_200 => "auto fast boxes for large VMF",
        PreviewDetailMode::Auto if solid_count > 350 => "auto faces, side-edge overlay skipped",
        PreviewDetailMode::Auto => "auto full detail",
        PreviewDetailMode::Full => "all faces and side edges",
    }
}

pub(crate) fn count_preview_deletions(
    preview: &PreviewDocument,
    criteria: &DeletionCriteria,
) -> PreviewDeletionCounts {
    if criteria.is_empty() {
        return PreviewDeletionCounts::default();
    }
    PreviewDeletionCounts {
        solids: preview
            .solids
            .iter()
            .filter(|solid| preview_solid_removed(solid, criteria))
            .count(),
        entities: preview
            .entities
            .iter()
            .filter(|entity| preview_entity_removed(entity, criteria))
            .count(),
    }
}

pub(crate) fn preview_entity_removed(
    entity: &PreviewEntityMarker,
    criteria: &DeletionCriteria,
) -> bool {
    if criteria.is_empty() || preview_entity_protected(entity.classname.as_deref(), criteria) {
        return false;
    }
    if criteria.drop_all_entities {
        return true;
    }
    entity
        .classname
        .as_ref()
        .map(|classname| criteria.classnames.contains(classname))
        .unwrap_or(false)
        || entity
            .targetname
            .as_ref()
            .map(|targetname| criteria.targetnames.contains(targetname))
            .unwrap_or(false)
}

pub(crate) fn preview_solid_removed(solid: &PreviewSolid, criteria: &DeletionCriteria) -> bool {
    if criteria.is_empty() || preview_entity_protected(solid.classname.as_deref(), criteria) {
        return false;
    }

    let entity_level_match = solid.owner_block == "entity"
        && (criteria.drop_all_entities
            || solid
                .classname
                .as_ref()
                .map(|classname| criteria.classnames.contains(classname))
                .unwrap_or(false)
            || solid
                .targetname
                .as_ref()
                .map(|targetname| criteria.targetnames.contains(targetname))
                .unwrap_or(false));
    if entity_level_match {
        return true;
    }

    if criteria.brush_roles.is_empty() {
        return false;
    }

    let role_match = solid
        .roles
        .iter()
        .any(|role| criteria.brush_roles.contains(role));
    if solid.owner_block == "world" {
        return role_match;
    }

    match criteria.brush_entity_mode {
        BrushEntityDeletionMode::WholeEntity => role_match,
        BrushEntityDeletionMode::MatchingSolids => {
            role_match || criteria.brush_roles.contains(&BrushRole::BrushEntity)
        }
    }
}

pub(crate) fn preview_entity_protected(
    classname: Option<&str>,
    criteria: &DeletionCriteria,
) -> bool {
    criteria.protect_critical_entities
        && classname.map(is_critical_entity_classname).unwrap_or(false)
}

pub(crate) fn deletion_entity_color(
    color: egui::Color32,
    mode: DeletionPreviewMode,
    removed: bool,
) -> egui::Color32 {
    if !removed {
        return color;
    }
    match mode {
        DeletionPreviewMode::Off => color,
        DeletionPreviewMode::HighlightRemoved => egui::Color32::from_rgb(255, 70, 70),
        DeletionPreviewMode::DimRemoved => color.gamma_multiply(0.25),
        DeletionPreviewMode::HideRemoved => color,
    }
}

pub(crate) fn nice_grid_step(scale: f32) -> f64 {
    let target_world = (72.0_f64 / scale.max(0.001) as f64).max(1.0);
    let base = 10_f64.powf(target_world.log10().floor());
    for multiplier in [1.0, 2.0, 4.0, 8.0, 16.0] {
        let candidate = base * multiplier;
        if candidate >= target_world {
            return candidate;
        }
    }
    base * 32.0
}
