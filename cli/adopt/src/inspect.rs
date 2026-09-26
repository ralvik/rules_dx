use super::AdoptError;

pub fn inspect_scope_allowed(scope: &str, external: bool) -> bool {
    !scope.is_empty() && !external
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectPlan {
    pub verb: String,
    pub expr: String,
}

pub fn plan_inspect(kind: &str, scope: &str, configured: bool) -> Result<InspectPlan, AdoptError> {
    if !inspect_scope_allowed(scope, scope.starts_with('@')) {
        return Err(AdoptError::RejectedScope {
            scope: scope.to_owned(),
        });
    }
    let verb = if configured { "cquery" } else { "query" };
    let expr = match kind {
        "owners" => format!("kind('rule', rdeps(//..., {scope}, 1))"),
        "deps" => format!("deps({scope})"),
        "why" => {
            return Err(AdoptError::WhyNeedsOwner);
        }
        _ => {
            return Err(AdoptError::UnknownInspect {
                kind: kind.to_owned(),
            });
        }
    };
    Ok(InspectPlan {
        verb: verb.to_owned(),
        expr,
    })
}

pub fn plan_somepath(from: &str, to: &str, configured: bool) -> Result<InspectPlan, AdoptError> {
    if !inspect_scope_allowed(from, from.starts_with('@')) {
        return Err(AdoptError::RejectedScope {
            scope: from.to_owned(),
        });
    }
    if !inspect_scope_allowed(to, to.starts_with('@')) {
        return Err(AdoptError::RejectedScope {
            scope: to.to_owned(),
        });
    }
    let verb = if configured { "cquery" } else { "query" };
    Ok(InspectPlan {
        verb: verb.to_owned(),
        expr: format!("somepath({from}, {to})"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_rejects_empty_and_external_scopes() {
        assert!(inspect_scope_allowed("//pkg:target", false));
        assert!(!inspect_scope_allowed("", false));
        assert!(!inspect_scope_allowed("//pkg:target", true));
        assert!(!inspect_scope_allowed("@other//pkg:target", true));
    }

    #[test]
    fn inspect_plans_query_forwarding() {
        let owners = plan_inspect("owners", "//a:one", false).expect("q");
        assert_eq!(owners.verb, "query");
        assert!(owners.expr.contains("rdeps"));
        assert!(!owners.expr.contains("query"));
        let deps = plan_inspect("deps", "//a:one", true).expect("q");
        assert_eq!(deps.verb, "cquery");
        assert!(deps.expr.contains("deps("));
        assert!(plan_inspect("owners", "@o//a:one", false).is_err());
        assert!(plan_inspect("why", "//a:one", false).is_err());
        let leg = plan_somepath("//a:one", "//b:two", false).expect("somepath");
        assert_eq!(leg.verb, "query");
        assert_eq!(leg.expr, "somepath(//a:one, //b:two)");
        assert!(plan_somepath("@o//a:one", "//b:two", false).is_err());
        assert!(plan_somepath("//a:one", "", false).is_err());
    }
}
