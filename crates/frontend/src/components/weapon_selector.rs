use dioxus::prelude::*;

use crate::api::WeaponData;

#[component]
pub fn WeaponSelector(weapons: Vec<WeaponData>, selected_weapon: Signal<String>) -> Element {
    let colonial: Vec<&WeaponData> = weapons
        .iter()
        .filter(|w| w.faction == "COLONIAL" || w.faction == "BOTH")
        .collect();
    let warden: Vec<&WeaponData> = weapons
        .iter()
        .filter(|w| w.faction == "WARDEN" || w.faction == "BOTH")
        .collect();

    rsx! {
        div { class: "panel",
            h3 { "Active Weapon" }
            select {
                "aria-label": "Select weapon",
                value: "{selected_weapon}",
                onchange: move |evt: Event<FormData>| {
                    selected_weapon.set(evt.value().to_string());
                },
                option { value: "", "-- Select Weapon --" }
                optgroup { label: "Colonial",
                    for w in colonial {
                        option {
                            value: "{w.slug}",
                            selected: *selected_weapon.read() == w.slug,
                            "{w.display_name} ({w.min_range}-{w.max_range}m)"
                        }
                    }
                }
                optgroup { label: "Warden",
                    for w in warden {
                        option {
                            value: "{w.slug}",
                            selected: *selected_weapon.read() == w.slug,
                            "{w.display_name} ({w.min_range}-{w.max_range}m)"
                        }
                    }
                }
            }
        }
    }
}
