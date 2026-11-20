//! Reading DMN XML into a raw (uncompiled) representation.

use roxmltree::{Document, Node};

#[derive(Debug, Clone)]
pub struct RawDefinitions {
    pub name: String,
    pub namespace: String,
    pub imports: Vec<RawImport>,
    pub item_defs: Vec<RawItemDef>,
    pub drg: Vec<RawDrg>,
}

#[derive(Debug, Clone)]
pub struct RawImport {
    pub name: String,
    pub namespace: String,
    pub location: Option<String>,
    pub import_type: Option<String>,
}

/// Function item: parameters (name, typeRef) and output typeRef.
pub type FunctionItem = (Vec<(String, Option<String>)>, Option<String>);

#[derive(Debug, Clone, Default)]
pub struct RawItemDef {
    pub name: String,
    pub type_ref: Option<String>,
    pub is_collection: bool,
    pub components: Vec<RawItemDef>,
    pub allowed_values: Option<String>,
    pub type_constraint: Option<String>,
    pub function_item: Option<FunctionItem>,
}

#[derive(Debug, Clone)]
pub enum RawReq {
    Input(String),
    Decision(String),
}

#[derive(Debug, Clone)]
pub enum RawDrg {
    InputData {
        id: String,
        name: String,
        type_ref: Option<String>,
    },
    Decision {
        id: String,
        name: String,
        type_ref: Option<String>,
        info_reqs: Vec<RawReq>,
        know_reqs: Vec<String>,
        logic: Option<RawExpr>,
    },
    Bkm {
        id: String,
        name: String,
        type_ref: Option<String>,
        know_reqs: Vec<String>,
        logic: Option<RawExpr>,
    },
    DecisionService {
        id: String,
        name: String,
        type_ref: Option<String>,
        outputs: Vec<String>,
        encapsulated: Vec<String>,
        input_decisions: Vec<String>,
        input_data: Vec<String>,
    },
}

impl RawDrg {
    pub fn id(&self) -> &str {
        match self {
            RawDrg::InputData { id, .. }
            | RawDrg::Decision { id, .. }
            | RawDrg::Bkm { id, .. }
            | RawDrg::DecisionService { id, .. } => id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            RawDrg::InputData { name, .. }
            | RawDrg::Decision { name, .. }
            | RawDrg::Bkm { name, .. }
            | RawDrg::DecisionService { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawContextEntry {
    pub name: Option<String>,
    pub type_ref: Option<String>,
    pub value: Option<RawExpr>,
}

#[derive(Debug, Clone)]
pub struct RawDtInput {
    pub expr: String,
    pub type_ref: Option<String>,
    pub input_values: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RawDtOutput {
    pub name: Option<String>,
    pub type_ref: Option<String>,
    pub output_values: Option<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RawDecisionTable {
    pub hit_policy: String,
    pub aggregation: Option<String>,
    pub inputs: Vec<RawDtInput>,
    pub outputs: Vec<RawDtOutput>,
    pub rules: Vec<(Vec<String>, Vec<String>)>,
}

#[derive(Debug, Clone)]
pub enum RawExprKind {
    Literal(Option<String>),
    Context(Vec<RawContextEntry>),
    DecisionTable(RawDecisionTable),
    Invocation {
        callee: Box<RawExpr>,
        bindings: Vec<(String, Option<RawExpr>)>,
    },
    FunctionDef {
        params: Vec<(String, Option<String>)>,
        kind: String,
        body: Option<Box<RawExpr>>,
    },
    Relation {
        columns: Vec<(String, Option<String>)>,
        rows: Vec<Vec<Option<RawExpr>>>,
    },
    List(Vec<RawExpr>),
    Conditional {
        cond: Box<RawExpr>,
        then: Box<RawExpr>,
        otherwise: Box<RawExpr>,
    },
    Filter {
        input: Box<RawExpr>,
        matcher: Box<RawExpr>,
    },
    For {
        var: String,
        input: Box<RawExpr>,
        ret: Box<RawExpr>,
    },
    Quantified {
        every: bool,
        var: String,
        input: Box<RawExpr>,
        cond: Box<RawExpr>,
    },
    UnaryTests(Option<String>),
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub struct RawExpr {
    pub kind: RawExprKind,
    pub type_ref: Option<String>,
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn attr(n: Node, name: &str) -> Option<String> {
    n.attribute(name).map(|s| s.to_string())
}

fn text_of(n: Node) -> Option<String> {
    child(n, "text").map(|t| t.children().filter_map(|c| c.text()).collect::<String>())
}

fn href(n: Node) -> Option<String> {
    n.attribute("href").map(|s| s.to_string())
}

const EXPR_KINDS: &[&str] = &[
    "literalExpression",
    "context",
    "decisionTable",
    "invocation",
    "functionDefinition",
    "relation",
    "list",
    "conditional",
    "filter",
    "for",
    "some",
    "every",
    "unaryTests",
];

fn is_expr(n: &Node) -> bool {
    n.is_element() && EXPR_KINDS.contains(&n.tag_name().name())
}

fn first_expr(n: Node) -> Option<RawExpr> {
    n.children().find(is_expr).map(read_expr)
}

fn boxed(n: Node, name: &str) -> Box<RawExpr> {
    Box::new(child(n, name).and_then(first_expr).unwrap_or(RawExpr {
        kind: RawExprKind::Literal(None),
        type_ref: None,
    }))
}

fn read_expr(n: Node) -> RawExpr {
    let type_ref = attr(n, "typeRef");
    let kind = match n.tag_name().name() {
        "literalExpression" => RawExprKind::Literal(text_of(n)),
        "unaryTests" => RawExprKind::UnaryTests(text_of(n)),
        "context" => RawExprKind::Context(
            children(n, "contextEntry")
                .map(|e| {
                    let var = child(e, "variable");
                    RawContextEntry {
                        name: var.and_then(|v| attr(v, "name")),
                        type_ref: var.and_then(|v| attr(v, "typeRef")),
                        value: first_expr(e),
                    }
                })
                .collect(),
        ),
        "decisionTable" => {
            let inputs = children(n, "input")
                .map(|i| {
                    let ie = child(i, "inputExpression");
                    RawDtInput {
                        expr: ie.and_then(text_of).unwrap_or_default(),
                        type_ref: ie.and_then(|x| attr(x, "typeRef")),
                        input_values: child(i, "inputValues").and_then(text_of),
                    }
                })
                .collect();
            let outputs = children(n, "output")
                .map(|o| RawDtOutput {
                    name: attr(o, "name"),
                    type_ref: attr(o, "typeRef"),
                    output_values: child(o, "outputValues").and_then(text_of),
                    default: child(o, "defaultOutputEntry").and_then(text_of),
                })
                .collect();
            let rules = children(n, "rule")
                .map(|r| {
                    (
                        children(r, "inputEntry")
                            .map(|e| text_of(e).unwrap_or_default())
                            .collect(),
                        children(r, "outputEntry")
                            .map(|e| text_of(e).unwrap_or_default())
                            .collect(),
                    )
                })
                .collect();
            RawExprKind::DecisionTable(RawDecisionTable {
                hit_policy: attr(n, "hitPolicy").unwrap_or_else(|| "UNIQUE".into()),
                aggregation: attr(n, "aggregation"),
                inputs,
                outputs,
                rules,
            })
        }
        "invocation" => {
            let callee = first_expr(n).unwrap_or(RawExpr {
                kind: RawExprKind::Literal(None),
                type_ref: None,
            });
            let bindings = children(n, "binding")
                .map(|b| {
                    let p = child(b, "parameter")
                        .and_then(|p| attr(p, "name"))
                        .unwrap_or_default();
                    (p, first_expr(b))
                })
                .collect();
            RawExprKind::Invocation {
                callee: Box::new(callee),
                bindings,
            }
        }
        "functionDefinition" => RawExprKind::FunctionDef {
            params: children(n, "formalParameter")
                .map(|p| (attr(p, "name").unwrap_or_default(), attr(p, "typeRef")))
                .collect(),
            kind: attr(n, "kind").unwrap_or_else(|| "FEEL".into()),
            body: first_expr(n).map(Box::new),
        },
        "relation" => {
            let columns = children(n, "column")
                .map(|c| (attr(c, "name").unwrap_or_default(), attr(c, "typeRef")))
                .collect();
            let rows = children(n, "row")
                .map(|r| {
                    r.children()
                        .filter(is_expr)
                        .map(|e| Some(read_expr(e)))
                        .collect()
                })
                .collect();
            RawExprKind::Relation { columns, rows }
        }
        "list" => RawExprKind::List(n.children().filter(is_expr).map(read_expr).collect()),
        "conditional" => RawExprKind::Conditional {
            cond: boxed(n, "if"),
            then: boxed(n, "then"),
            otherwise: boxed(n, "else"),
        },
        "filter" => RawExprKind::Filter {
            input: boxed(n, "in"),
            matcher: boxed(n, "match"),
        },
        "for" => RawExprKind::For {
            var: attr(n, "iteratorVariable").unwrap_or_default(),
            input: boxed(n, "in"),
            ret: boxed(n, "return"),
        },
        "some" | "every" => RawExprKind::Quantified {
            every: n.tag_name().name() == "every",
            var: attr(n, "iteratorVariable").unwrap_or_default(),
            input: boxed(n, "in"),
            cond: boxed(n, "satisfies"),
        },
        other => RawExprKind::Unsupported(other.to_string()),
    };
    RawExpr { kind, type_ref }
}

fn read_item_def(n: Node) -> RawItemDef {
    RawItemDef {
        name: attr(n, "name").unwrap_or_default(),
        type_ref: child(n, "typeRef").map(|t| t.text().unwrap_or("").trim().to_string()),
        is_collection: n.attribute("isCollection") == Some("true"),
        components: children(n, "itemComponent").map(read_item_def).collect(),
        allowed_values: child(n, "allowedValues").and_then(text_of),
        type_constraint: child(n, "typeConstraint").and_then(text_of),
        function_item: child(n, "functionItem").map(|f| {
            (
                children(f, "parameters")
                    .map(|p| (attr(p, "name").unwrap_or_default(), attr(p, "typeRef")))
                    .collect(),
                attr(f, "outputTypeRef"),
            )
        }),
    }
}

fn var_type(n: Node) -> Option<String> {
    child(n, "variable").and_then(|v| attr(v, "typeRef"))
}

pub fn read_definitions(xml: &str) -> Result<RawDefinitions, String> {
    let doc = Document::parse(xml).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    if root.tag_name().name() != "definitions" {
        return Err("not a DMN definitions document".into());
    }
    let mut defs = RawDefinitions {
        name: attr(root, "name").unwrap_or_default(),
        namespace: attr(root, "namespace").unwrap_or_default(),
        imports: Vec::new(),
        item_defs: Vec::new(),
        drg: Vec::new(),
    };
    for n in root.children().filter(|c| c.is_element()) {
        let id = attr(n, "id").unwrap_or_default();
        let name = attr(n, "name").unwrap_or_default();
        match n.tag_name().name() {
            "import" => defs.imports.push(RawImport {
                name: attr(n, "name").unwrap_or_default(),
                namespace: attr(n, "namespace").unwrap_or_default(),
                location: attr(n, "locationURI"),
                import_type: attr(n, "importType"),
            }),
            "itemDefinition" => defs.item_defs.push(read_item_def(n)),
            "inputData" => defs.drg.push(RawDrg::InputData {
                id,
                name,
                type_ref: var_type(n),
            }),
            "decision" => {
                let mut info_reqs = Vec::new();
                for ir in children(n, "informationRequirement") {
                    if let Some(r) = child(ir, "requiredInput").and_then(href) {
                        info_reqs.push(RawReq::Input(r));
                    } else if let Some(r) = child(ir, "requiredDecision").and_then(href) {
                        info_reqs.push(RawReq::Decision(r));
                    }
                }
                let know_reqs = children(n, "knowledgeRequirement")
                    .filter_map(|k| child(k, "requiredKnowledge").and_then(href))
                    .collect();
                let logic = first_expr(n);
                defs.drg.push(RawDrg::Decision {
                    id,
                    name,
                    type_ref: var_type(n),
                    info_reqs,
                    know_reqs,
                    logic,
                });
            }
            "businessKnowledgeModel" => {
                let know_reqs = children(n, "knowledgeRequirement")
                    .filter_map(|k| child(k, "requiredKnowledge").and_then(href))
                    .collect();
                let logic = child(n, "encapsulatedLogic").map(|e| {
                    let mut x = read_expr(e);
                    if let RawExprKind::Unsupported(_) = x.kind {
                        x = RawExpr {
                            kind: RawExprKind::FunctionDef {
                                params: children(e, "formalParameter")
                                    .map(|p| {
                                        (attr(p, "name").unwrap_or_default(), attr(p, "typeRef"))
                                    })
                                    .collect(),
                                kind: attr(e, "kind").unwrap_or_else(|| "FEEL".into()),
                                body: first_expr(e).map(Box::new),
                            },
                            type_ref: attr(e, "typeRef"),
                        };
                    }
                    x
                });
                defs.drg.push(RawDrg::Bkm {
                    id,
                    name,
                    type_ref: var_type(n),
                    know_reqs,
                    logic,
                });
            }
            "decisionService" => {
                let hrefs = |tag: &str| children(n, tag).filter_map(href).collect::<Vec<_>>();
                defs.drg.push(RawDrg::DecisionService {
                    id,
                    name,
                    type_ref: var_type(n),
                    outputs: hrefs("outputDecision"),
                    encapsulated: hrefs("encapsulatedDecision"),
                    input_decisions: hrefs("inputDecision"),
                    input_data: hrefs("inputData"),
                });
            }
            _ => {}
        }
    }
    Ok(defs)
}
