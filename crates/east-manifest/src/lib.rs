#![forbid(unsafe_code)]
//! Manifest data model, YAML parsing, and import resolution for east.

pub mod error;
mod model;
pub mod path_resolve;
mod resolve;

pub use model::{
    CommandArg, CommandDecl, Defaults, Import, Manifest, ManifestSelf, Project, Remote,
};

#[cfg(test)]
mod tests {
    use super::*;

    // ── Remote ──────────────────────────────────────────────────────

    #[test]
    fn remote_new() {
        let r = Remote {
            name: "origin".into(),
            url_base: "https://github.com/my-org".into(),
        };
        assert_eq!(r.name, "origin");
        assert_eq!(r.url_base, "https://github.com/my-org");
    }

    #[test]
    fn remote_serde_round_trip() {
        let r = Remote {
            name: "origin".into(),
            url_base: "https://github.com/my-org".into(),
        };
        let yaml = serde_yaml::to_string(&r).unwrap();
        let r2: Remote = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(r.name, r2.name);
        assert_eq!(r.url_base, r2.url_base);
    }

    // ── Defaults ────────────────────────────────────────────────────

    #[test]
    fn defaults_all_none() {
        let d = Defaults {
            remote: None,
            revision: None,
        };
        assert!(d.remote.is_none());
        assert!(d.revision.is_none());
    }

    #[test]
    fn defaults_with_values() {
        let d = Defaults {
            remote: Some("origin".into()),
            revision: Some("main".into()),
        };
        assert_eq!(d.remote.as_deref(), Some("origin"));
        assert_eq!(d.revision.as_deref(), Some("main"));
    }

    #[test]
    fn defaults_serde_round_trip() {
        let d = Defaults {
            remote: Some("origin".into()),
            revision: Some("v1.0".into()),
        };
        let yaml = serde_yaml::to_string(&d).unwrap();
        let d2: Defaults = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(d.remote, d2.remote);
        assert_eq!(d.revision, d2.revision);
    }

    #[test]
    fn defaults_missing_fields_deserialize_as_none() {
        let yaml = "{}";
        let d: Defaults = serde_yaml::from_str(yaml).unwrap();
        assert!(d.remote.is_none());
        assert!(d.revision.is_none());
    }

    // ── Project ─────────────────────────────────────────────────────

    #[test]
    fn project_minimal() {
        let p = Project {
            name: "sdk-core".into(),
            path: None,
            remote: None,
            revision: None,
            groups: Vec::new(),
        };
        assert_eq!(p.name, "sdk-core");
        assert!(p.path.is_none());
        assert!(p.remote.is_none());
        assert!(p.revision.is_none());
        assert!(p.groups.is_empty());
    }

    #[test]
    fn project_full() {
        let p = Project {
            name: "sdk-core".into(),
            path: Some("sdk/core".into()),
            remote: Some("origin".into()),
            revision: Some("v1.2.0".into()),
            groups: vec!["required".into()],
        };
        assert_eq!(p.path.as_deref(), Some("sdk/core"));
        assert_eq!(p.revision.as_deref(), Some("v1.2.0"));
        assert_eq!(p.groups, vec!["required"]);
    }

    #[test]
    fn project_effective_path_defaults_to_name() {
        let p = Project {
            name: "sdk-core".into(),
            path: None,
            remote: None,
            revision: None,
            groups: Vec::new(),
        };
        assert_eq!(p.effective_path(), "sdk-core");
    }

    #[test]
    fn project_effective_path_uses_explicit_path() {
        let p = Project {
            name: "sdk-core".into(),
            path: Some("sdk/core".into()),
            remote: None,
            revision: None,
            groups: Vec::new(),
        };
        assert_eq!(p.effective_path(), "sdk/core");
    }

    #[test]
    fn project_serde_round_trip() {
        let p = Project {
            name: "sdk-core".into(),
            path: Some("sdk/core".into()),
            remote: Some("origin".into()),
            revision: Some("v1.2.0".into()),
            groups: vec!["required".into(), "hal".into()],
        };
        let yaml = serde_yaml::to_string(&p).unwrap();
        let p2: Project = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(p.name, p2.name);
        assert_eq!(p.path, p2.path);
        assert_eq!(p.remote, p2.remote);
        assert_eq!(p.revision, p2.revision);
        assert_eq!(p.groups, p2.groups);
    }

    #[test]
    fn project_missing_optional_fields() {
        let yaml = "name: my-lib";
        let p: Project = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(p.name, "my-lib");
        assert!(p.path.is_none());
        assert!(p.remote.is_none());
        assert!(p.revision.is_none());
        assert!(p.groups.is_empty());
    }

    // ── Import ──────────────────────────────────────────────────────

    #[test]
    fn import_minimal() {
        let i = Import {
            file: "sdk/core/east.yml".into(),
            allowlist: Vec::new(),
        };
        assert_eq!(i.file, "sdk/core/east.yml");
        assert!(i.allowlist.is_empty());
    }

    #[test]
    fn import_with_allowlist() {
        let i = Import {
            file: "sdk/core/east.yml".into(),
            allowlist: vec!["hal-*".into()],
        };
        assert_eq!(i.allowlist, vec!["hal-*"]);
    }

    #[test]
    fn import_serde_round_trip() {
        let i = Import {
            file: "sub/east.yml".into(),
            allowlist: vec!["hal-*".into(), "driver-*".into()],
        };
        let yaml = serde_yaml::to_string(&i).unwrap();
        let i2: Import = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(i.file, i2.file);
        assert_eq!(i.allowlist, i2.allowlist);
    }

    #[test]
    fn import_missing_allowlist_defaults_to_empty() {
        let yaml = "file: sub/east.yml";
        let i: Import = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(i.file, "sub/east.yml");
        assert!(i.allowlist.is_empty());
    }

    // ── Manifest ────────────────────────────────────────────────────

    #[test]
    fn manifest_minimal() {
        let m = Manifest {
            version: 1,
            remotes: Vec::new(),
            defaults: None,
            projects: Vec::new(),
            imports: Vec::new(),
            group_filter: Vec::new(),
            commands: Vec::new(),
            manifest_self: None,
        };
        assert_eq!(m.version, 1);
        assert!(m.remotes.is_empty());
        assert!(m.defaults.is_none());
        assert!(m.projects.is_empty());
        assert!(m.imports.is_empty());
        assert!(m.group_filter.is_empty());
    }

    #[test]
    fn manifest_full_construction() {
        let m = Manifest {
            version: 1,
            remotes: vec![Remote {
                name: "origin".into(),
                url_base: "https://github.com/org".into(),
            }],
            defaults: Some(Defaults {
                remote: Some("origin".into()),
                revision: Some("main".into()),
            }),
            projects: vec![
                Project {
                    name: "sdk-core".into(),
                    path: Some("sdk/core".into()),
                    remote: None,
                    revision: Some("v1.2.0".into()),
                    groups: vec!["required".into()],
                },
                Project {
                    name: "sdk-drivers".into(),
                    path: Some("sdk/drivers".into()),
                    remote: None,
                    revision: None,
                    groups: vec!["required".into()],
                },
            ],
            imports: vec![Import {
                file: "sdk/core/east.yml".into(),
                allowlist: vec!["hal-*".into()],
            }],
            group_filter: vec!["+required".into(), "-optional".into()],
            commands: Vec::new(),
            manifest_self: None,
        };
        assert_eq!(m.remotes.len(), 1);
        assert_eq!(m.projects.len(), 2);
        assert_eq!(m.imports.len(), 1);
        assert_eq!(m.group_filter, vec!["+required", "-optional"]);
    }

    #[test]
    fn manifest_serde_round_trip() {
        let m = Manifest {
            version: 1,
            remotes: vec![Remote {
                name: "origin".into(),
                url_base: "https://github.com/org".into(),
            }],
            defaults: Some(Defaults {
                remote: Some("origin".into()),
                revision: Some("main".into()),
            }),
            projects: vec![Project {
                name: "sdk-core".into(),
                path: None,
                remote: None,
                revision: None,
                groups: Vec::new(),
            }],
            imports: Vec::new(),
            group_filter: Vec::new(),
            commands: Vec::new(),
            manifest_self: None,
        };
        let yaml = serde_yaml::to_string(&m).unwrap();
        let m2: Manifest = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(m.version, m2.version);
        assert_eq!(m.remotes.len(), m2.remotes.len());
        assert_eq!(m.projects.len(), m2.projects.len());
    }

    #[test]
    fn manifest_missing_optional_fields() {
        let yaml = "version: 1";
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.version, 1);
        assert!(m.remotes.is_empty());
        assert!(m.defaults.is_none());
        assert!(m.projects.is_empty());
        assert!(m.imports.is_empty());
        assert!(m.group_filter.is_empty());
    }

    #[test]
    fn manifest_version_must_be_present() {
        let yaml = "projects: []";
        let result = serde_yaml::from_str::<Manifest>(yaml);
        assert!(result.is_err());
    }

    // ── YAML parsing (commit 6) ─────────────────────────────────────

    #[test]
    fn parse_full_east_yml() {
        let yaml = r"
version: 1

remotes:
  - name: origin
    url-base: https://github.com/my-org

defaults:
  remote: origin
  revision: main

projects:
  - name: sdk-core
    path: sdk/core
    revision: v1.2.0
    groups: [required]
  - name: sdk-drivers
    path: sdk/drivers
    groups: [required]
  - name: sdk-examples
    groups: [optional]

imports:
  - file: sdk/core/east.yml
    allowlist: [hal-*]

group-filter: [+required, -optional]
";
        let m = Manifest::from_yaml_str(yaml).unwrap();
        assert_eq!(m.version, 1);
        assert_eq!(m.remotes.len(), 1);
        assert_eq!(m.remotes[0].name, "origin");
        assert_eq!(m.remotes[0].url_base, "https://github.com/my-org");
        assert_eq!(
            m.defaults.as_ref().unwrap().remote.as_deref(),
            Some("origin")
        );
        assert_eq!(
            m.defaults.as_ref().unwrap().revision.as_deref(),
            Some("main")
        );
        assert_eq!(m.projects.len(), 3);
        assert_eq!(m.projects[0].name, "sdk-core");
        assert_eq!(m.projects[0].effective_path(), "sdk/core");
        assert_eq!(m.projects[1].effective_path(), "sdk/drivers");
        assert_eq!(m.projects[2].effective_path(), "sdk-examples");
        assert_eq!(m.imports.len(), 1);
        assert_eq!(m.group_filter, vec!["+required", "-optional"]);
    }

    #[test]
    fn parse_minimal_east_yml() {
        let yaml = "version: 1\n";
        let m = Manifest::from_yaml_str(yaml).unwrap();
        assert_eq!(m.version, 1);
        assert!(m.projects.is_empty());
    }

    #[test]
    fn parse_rejects_unsupported_version() {
        let yaml = "version: 99\n";
        let err = Manifest::from_yaml_str(yaml).unwrap_err();
        assert!(
            err.to_string().contains("unsupported"),
            "error should mention unsupported version: {err}"
        );
    }

    #[test]
    fn parse_rejects_duplicate_project_names() {
        let yaml = r"
version: 1
projects:
  - name: foo
  - name: foo
";
        let err = Manifest::from_yaml_str(yaml).unwrap_err();
        assert!(
            err.to_string().contains("duplicate"),
            "error should mention duplicate: {err}"
        );
    }

    #[test]
    fn parse_rejects_unknown_remote_in_project() {
        let yaml = r"
version: 1
remotes:
  - name: origin
    url-base: https://example.com
projects:
  - name: foo
    remote: nonexistent
";
        let err = Manifest::from_yaml_str(yaml).unwrap_err();
        assert!(
            err.to_string().contains("nonexistent"),
            "error should mention unknown remote: {err}"
        );
    }

    #[test]
    fn parse_rejects_unknown_remote_in_defaults() {
        let yaml = r"
version: 1
remotes:
  - name: origin
    url-base: https://example.com
defaults:
  remote: ghost
";
        let err = Manifest::from_yaml_str(yaml).unwrap_err();
        assert!(
            err.to_string().contains("ghost"),
            "error should mention unknown remote: {err}"
        );
    }

    // ── Group filtering ─────────────────────────────────────────────

    #[test]
    fn group_filter_includes_required_excludes_optional() {
        let m = Manifest {
            version: 1,
            remotes: Vec::new(),
            defaults: None,
            projects: vec![
                Project {
                    name: "a".into(),
                    path: None,
                    remote: None,
                    revision: None,
                    groups: vec!["required".into()],
                },
                Project {
                    name: "b".into(),
                    path: None,
                    remote: None,
                    revision: None,
                    groups: vec!["optional".into()],
                },
                Project {
                    name: "c".into(),
                    path: None,
                    remote: None,
                    revision: None,
                    groups: Vec::new(), // no group — always included
                },
            ],
            imports: Vec::new(),
            group_filter: vec!["+required".into(), "-optional".into()],
            commands: Vec::new(),
            manifest_self: None,
        };
        let filtered = m.filtered_projects();
        let names: Vec<&str> = filtered.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a", "c"]);
    }

    #[test]
    fn group_filter_empty_includes_all() {
        let m = Manifest {
            version: 1,
            remotes: Vec::new(),
            defaults: None,
            projects: vec![
                Project {
                    name: "a".into(),
                    path: None,
                    remote: None,
                    revision: None,
                    groups: vec!["optional".into()],
                },
                Project {
                    name: "b".into(),
                    path: None,
                    remote: None,
                    revision: None,
                    groups: Vec::new(),
                },
            ],
            imports: Vec::new(),
            group_filter: Vec::new(),
            commands: Vec::new(),
            manifest_self: None,
        };
        let filtered = m.filtered_projects();
        assert_eq!(filtered.len(), 2);
    }

    // ── URL construction ────────────────────────────────────────────

    #[test]
    fn project_clone_url_from_remote() {
        let m = Manifest {
            version: 1,
            remotes: vec![Remote {
                name: "origin".into(),
                url_base: "https://github.com/org".into(),
            }],
            defaults: Some(Defaults {
                remote: Some("origin".into()),
                revision: Some("main".into()),
            }),
            projects: vec![Project {
                name: "sdk-core".into(),
                path: None,
                remote: None,
                revision: None,
                groups: Vec::new(),
            }],
            imports: Vec::new(),
            group_filter: Vec::new(),
            commands: Vec::new(),
            manifest_self: None,
        };
        let url = m.project_clone_url(&m.projects[0]).unwrap();
        assert_eq!(url, "https://github.com/org/sdk-core");
    }

    #[test]
    fn project_clone_url_explicit_remote_overrides_default() {
        let m = Manifest {
            version: 1,
            remotes: vec![
                Remote {
                    name: "origin".into(),
                    url_base: "https://github.com/org".into(),
                },
                Remote {
                    name: "mirror".into(),
                    url_base: "https://mirror.example.com".into(),
                },
            ],
            defaults: Some(Defaults {
                remote: Some("origin".into()),
                revision: None,
            }),
            projects: vec![Project {
                name: "sdk-core".into(),
                path: None,
                remote: Some("mirror".into()),
                revision: None,
                groups: Vec::new(),
            }],
            imports: Vec::new(),
            group_filter: Vec::new(),
            commands: Vec::new(),
            manifest_self: None,
        };
        let url = m.project_clone_url(&m.projects[0]).unwrap();
        assert_eq!(url, "https://mirror.example.com/sdk-core");
    }

    #[test]
    fn project_effective_revision_falls_back_to_defaults() {
        let m = Manifest {
            version: 1,
            remotes: Vec::new(),
            defaults: Some(Defaults {
                remote: None,
                revision: Some("main".into()),
            }),
            projects: vec![Project {
                name: "a".into(),
                path: None,
                remote: None,
                revision: None,
                groups: Vec::new(),
            }],
            imports: Vec::new(),
            group_filter: Vec::new(),
            commands: Vec::new(),
            manifest_self: None,
        };
        let rev = m.project_revision(&m.projects[0]);
        assert_eq!(rev, Some("main"));
    }

    #[test]
    fn project_explicit_revision_overrides_default() {
        let m = Manifest {
            version: 1,
            remotes: Vec::new(),
            defaults: Some(Defaults {
                remote: None,
                revision: Some("main".into()),
            }),
            projects: vec![Project {
                name: "a".into(),
                path: None,
                remote: None,
                revision: Some("v2.0".into()),
                groups: Vec::new(),
            }],
            imports: Vec::new(),
            group_filter: Vec::new(),
            commands: Vec::new(),
            manifest_self: None,
        };
        let rev = m.project_revision(&m.projects[0]);
        assert_eq!(rev, Some("v2.0"));
    }
}
