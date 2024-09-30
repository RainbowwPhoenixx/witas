use egui::Event;

/// Create a DragValue with additionnal scroll interactions
/// Press shift to midify 10x faster, and control for 100x. These stack.
pub fn scrollable_dragvalue(value: &mut u32) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| {
        let mut dragvalue = ui.add(egui::DragValue::new(value));

        let events = dragvalue.ctx.input(|i| i.events.clone());

        if dragvalue.hovered() {
            for event in events {
                if let Event::MouseWheel {
                    unit: _,
                    delta,
                    modifiers,
                } = event
                {
                    let mut multiplier = 1;
                    if modifiers.shift {
                        multiplier *= 10;
                    }
                    if modifiers.ctrl {
                        multiplier *= 100;
                    }

                    *value = value.saturating_add_signed(multiplier * delta.y as i32);
                    dragvalue.mark_changed();
                }
            }
        }

        dragvalue
    }
}

// pub fn file_combobox(value: &mut String, folder: String) -> impl egui::Widget + '_ {
//     let options = vec![
//         "7lasersv2.wtas".to_string(),
//         "tmp.wtas".to_string(),
//         "7lasers.wtas".to_string(),
//     ];

//     // egui::ComboBox::from_label("").show_ui(ui, |ui| {
//     //     for option in options {
//     //         ui.selectable_value(current_value, selected_value, text)
//     //     }
//     // });
// }
