use std::collections::{HashMap, hash_map::Entry};

use indexmap::IndexMap;

use super::ui::{Finding, FindingKind, HostMapping};
use crate::linux::{groupname_to_id, username_to_id};
use crate::lxc::Config;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct State {
    pub is_running: bool,
    pub findings: Vec<Finding>,
    pub selected_finding: Option<usize>,
    pub host_mapping: HostMapping,
    pub lxc_configs: IndexMap<String, Config>,
    pub show_fix_popup: bool,
    pub show_settings_page: bool,
    pub show_logs_page: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_running: true,
            findings: Vec::new(),
            selected_finding: None,
            host_mapping: HostMapping {
                subuid: Vec::new(),
                subgid: Vec::new(),
            },
            lxc_configs: IndexMap::new(),
            show_fix_popup: false,
            show_settings_page: false,
            show_logs_page: false,
        }
    }
}

impl State {
    /// Findings are re-evaluated based on latest update
    pub fn evaluate_findings(&mut self) {
        self.findings.clear();

        let mut username_to_id_map = HashMap::new();
        let mut groupname_to_id_map = HashMap::new();
        let mut usernames = HashMap::new();
        let mut groupnames = HashMap::new();

        for (i, mapping) in self.host_mapping.subuid.iter().enumerate() {
            match usernames.entry(&mapping.host_user_id) {
                Entry::Occupied(occupancy) => {
                    let j = *occupancy.get();

                    self.findings.push(Finding {
                        kind: FindingKind::Bad,
                        message: "Cannot have multiple entries for the same user",
                        host_mapping_highlights: vec![j, i],
                        lxc_config_mapping_highlights: Vec::new(),
                    });
                },
                Entry::Vacant(vacancy) => {
                    vacancy.insert(i);
                },
            };
        }

        for (i, mapping) in self.host_mapping.subgid.iter().enumerate() {
            // Offset by the number of preceding gid entries
            let i = i + self.host_mapping.subuid.len();

            match groupnames.entry(&mapping.host_user_id) {
                Entry::Occupied(occupancy) => {
                    let j = *occupancy.get();

                    self.findings.push(Finding {
                        kind: FindingKind::Bad,
                        message: "Cannot have multiple entries for the same group",
                        host_mapping_highlights: vec![j, i],
                        lxc_config_mapping_highlights: Vec::new(),
                    });
                },
                Entry::Vacant(vacancy) => {
                    vacancy.insert(i);
                },
            };
        }

        for (i, (_filename, config)) in self.lxc_configs.iter().enumerate() {
            for (j, idmap) in config.sectionless_idmap().enumerate() {
                let cfg_pos = i + j;
                let mut idmap = idmap.trim().split(' ');
                let Some(kind) = idmap.next() else {
                    unreachable!("Invalid ID map entry kind");
                };
                let Some(host_id) = idmap.next() else {
                    unreachable!("Invalid ID map entry host user id");
                };
                let parsed_host_id = host_id.parse::<u32>().unwrap();
                let Some(host_sub_id) = idmap.next() else {
                    unreachable!("Invalid ID map entry host sub id");
                };
                let parsed_host_sub_id = host_sub_id.parse::<u32>().unwrap();
                let Some(host_sub_id_size) = idmap.next() else {
                    unreachable!("Invalid ID map entry host sub id count");
                };
                let parsed_host_sub_id_size = host_sub_id_size.parse::<u32>().unwrap();
                let (idmap, mappings, to_id) = if kind == "u" {
                    (
                        &mut username_to_id_map,
                        &*self.host_mapping.subuid,
                        username_to_id as fn(&str) -> color_eyre::Result<u32>,
                    )
                } else if kind == "g" {
                    (
                        &mut groupname_to_id_map,
                        &*self.host_mapping.subgid,
                        groupname_to_id as _,
                    )
                } else {
                    unreachable!("Invalid sub id kind")
                };

                for (k, mapping) in mappings.iter().enumerate() {
                    let subid_pos = if kind == "u" {
                        k
                    } else {
                        k + self.host_mapping.subuid.len()
                    };
                    let host_id = match idmap.entry(&mapping.host_user_id) {
                        Entry::Occupied(id) => *id.get(),
                        Entry::Vacant(vacancy) => *vacancy.insert(to_id(&mapping.host_user_id).expect("fixme")),
                    };

                    if host_id != parsed_host_id {
                        continue;
                    }

                    if parsed_host_sub_id < mapping.host_sub_id
                        || parsed_host_sub_id >= mapping.host_sub_id + mapping.host_sub_id_count
                        || parsed_host_sub_id + parsed_host_sub_id_size
                            >= mapping.host_sub_id + mapping.host_sub_id_count
                    {
                        self.findings.push(Finding {
                            kind: FindingKind::Bad,
                            message: "LXC config's host sub id range outside of host mapping range",
                            host_mapping_highlights: vec![subid_pos],
                            lxc_config_mapping_highlights: vec![cfg_pos],
                        });
                    }
                }
            }
        }
    }
}
