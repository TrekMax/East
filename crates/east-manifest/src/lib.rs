#![forbid(unsafe_code)]
//! Manifest data model, YAML parsing, and import resolution for east.

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
}
