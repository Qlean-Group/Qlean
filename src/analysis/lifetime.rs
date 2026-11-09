use std::collections::{HashMap, HashSet};
use qsc_data_structures::span::Span;
use qsc_frontend::compile::{CompileUnit, PackageStore};
use qsc_hir::hir::{
    Block, CallableDecl, Expr, ExprKind, Ident, ItemId, ItemKind, NodeId, Pat, PatKind, Res, Stmt,
    StmtKind, UnOp,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QubitKind {
    Fresh,
    Dirty,
    Parameter,
}

impl std::fmt::Display for QubitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QubitKind::Fresh => write!(f, "fresh (use)"),
            QubitKind::Dirty => write!(f, "dirty (borrow)"),
            QubitKind::Parameter => write!(f, "parameter"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QubitState {
    Clean,
    Dirty,
    Measured,
    Released,
}

impl std::fmt::Display for QubitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QubitState::Clean => write!(f, "Clean"),
            QubitState::Dirty => write!(f, "Dirty"),
            QubitState::Measured => write!(f, "Measured"),
            QubitState::Released => write!(f, "Released"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QubitId {
    pub node_id: NodeId,
    pub name: String,
    pub span: (usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionKind {
    Allocation(QubitKind),
    Gate(String),
    Measurement(String),
    Reset(String),
    Release,
}

impl std::fmt::Display for TransitionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionKind::Allocation(k) => write!(f, "allocate {}", k),
            TransitionKind::Gate(g) => write!(f, "gate {}", g),
            TransitionKind::Measurement(m) => write!(f, "measure {}", m),
            TransitionKind::Reset(r) => write!(f, "reset {}", r),
            TransitionKind::Release => write!(f, "release"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QubitTransition {
    pub step: usize,
    pub kind: TransitionKind,
    pub span: (usize, usize),
    pub details: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateApplication {
    pub gate_name: String,
    pub qubits: Vec<NodeId>,
    pub is_adjoint: bool,
    pub span: (usize, usize),
    pub step: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QubitLifetime {
    pub qubit: QubitId,
    pub kind: QubitKind,
    pub state: QubitState,
    pub transitions: Vec<QubitTransition>,
    pub has_leak: bool,
    pub missing_uncompute: bool,
    pub release_span: Option<(usize, usize)>,
    pub gate_history: Vec<GateApplication>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntanglementEvent {
    pub measurement_span: (usize, usize),
    pub condition_span: (usize, usize),
    pub action_span: (usize, usize),
    pub source_qubit: String,
    pub target_qubit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationTrace {
    pub operation_name: String,
    pub operation_span: (usize, usize),
    pub lifetimes: Vec<QubitLifetime>,
    pub entanglements: Vec<EntanglementEvent>,
}

pub struct LifetimeTracker<'a> {
    store: &'a PackageStore,
    unit: &'a CompileUnit,
    aliases: HashMap<NodeId, NodeId>,
    lifetimes: Vec<QubitLifetime>,
    measurement_results: HashMap<NodeId, (String, (usize, usize))>,
    entanglements: Vec<EntanglementEvent>,
    step_counter: usize,
}

impl<'a> LifetimeTracker<'a> {
    pub fn new(store: &'a PackageStore, unit: &'a CompileUnit) -> Self {
        Self {
            store,
            unit,
            aliases: HashMap::new(),
            lifetimes: Vec::new(),
            measurement_results: HashMap::new(),
            entanglements: Vec::new(),
            step_counter: 0,
        }
    }

    pub fn trace_operation(mut self, decl: &CallableDecl) -> OperationTrace {
        let param_idents = collect_pat_idents(&decl.input);
        for ident in param_idents {
            let qubit_id = QubitId {
                node_id: ident.id,
                name: ident.name.to_string(),
                span: (ident.span.lo as usize, ident.span.hi as usize),
            };
            self.lifetimes.push(QubitLifetime {
                qubit: qubit_id,
                kind: QubitKind::Parameter,
                state: QubitState::Dirty,
                transitions: Vec::new(),
                has_leak: false,
                missing_uncompute: false,
                release_span: None,
                gate_history: Vec::new(),
            });
        }

        if let qsc_hir::hir::SpecBody::Impl(_, block) = &decl.body.body {
            self.visit_block(block, None);
        }

        for lifetime in &mut self.lifetimes {
            if lifetime.kind == QubitKind::Fresh
                && lifetime.state != QubitState::Clean
                && lifetime.state != QubitState::Released
            {
                lifetime.has_leak = true;
            }
        }

        OperationTrace {
            operation_name: decl.name.name.to_string(),
            operation_span: (decl.span.lo as usize, decl.span.hi as usize),
            lifetimes: self.lifetimes,
            entanglements: self.entanglements,
        }
    }

    fn resolve_node_id(&self, node_id: NodeId) -> NodeId {
        let mut curr = node_id;
        while let Some(&next) = self.aliases.get(&curr) {
            curr = next;
        }
        curr
    }

    fn find_lifetime_mut(&mut self, node_id: NodeId) -> Option<&mut QubitLifetime> {
        let canonical = self.resolve_node_id(node_id);
        self.lifetimes.iter_mut().find(|l| l.qubit.node_id == canonical)
    }

    fn find_lifetime(&self, node_id: NodeId) -> Option<&QubitLifetime> {
        let canonical = self.resolve_node_id(node_id);
        self.lifetimes.iter().find(|l| l.qubit.node_id == canonical)
    }

    fn visit_block(&mut self, block: &Block, active_measurement_sources: Option<&HashSet<String>>) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt, active_measurement_sources);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt, active_measurement_sources: Option<&HashSet<String>>) {
        match &stmt.kind {
            StmtKind::Local(_, pat, init) => {
                let init_call = self.as_call_info(init);
                if let Some((callee_name, _)) = &init_call {
                    if is_qubit_allocator(callee_name) {
                        let kind = if callee_name.contains("borrow") {
                            QubitKind::Dirty
                        } else {
                            QubitKind::Fresh
                        };
                        let initial_state = match kind {
                            QubitKind::Fresh => QubitState::Clean,
                            QubitKind::Dirty => QubitState::Dirty,
                            QubitKind::Parameter => QubitState::Dirty,
                        };
                        let idents = collect_pat_idents(pat);
                        for ident in idents {
                            self.step_counter += 1;
                            let qubit_id = QubitId {
                                node_id: ident.id,
                                name: ident.name.to_string(),
                                span: (ident.span.lo as usize, ident.span.hi as usize),
                            };
                            let trans = QubitTransition {
                                step: self.step_counter,
                                kind: TransitionKind::Allocation(kind),
                                span: (ident.span.lo as usize, ident.span.hi as usize),
                                details: format!("Allocated as {}", kind),
                            };
                            self.lifetimes.push(QubitLifetime {
                                qubit: qubit_id,
                                kind,
                                state: initial_state,
                                transitions: vec![trans],
                                has_leak: false,
                                missing_uncompute: false,
                                release_span: None,
                                gate_history: Vec::new(),
                            });
                        }
                        return;
                    }

                    if is_measurement(callee_name) {
                        let qubits = self.extract_qubit_refs(&init_call.as_ref().unwrap().1);
                        let idents = collect_pat_idents(pat);
                        for qubit_node in &qubits {
                            let q_name = self
                                .find_lifetime(*qubit_node)
                                .map(|l| l.qubit.name.clone())
                                .unwrap_or_else(|| "qubit".to_string());
                            for res_ident in &idents {
                                self.measurement_results.insert(
                                    res_ident.id,
                                    (
                                        q_name.clone(),
                                        (init.span.lo as usize, init.span.hi as usize),
                                    ),
                                );
                            }
                        }
                    }
                }

                if let ExprKind::Var(Res::Local(src_node), _) = &init.kind {
                    let idents = collect_pat_idents(pat);
                    for ident in idents {
                        self.aliases.insert(ident.id, *src_node);
                    }
                }

                self.visit_expr(init, active_measurement_sources);
            }
            StmtKind::Semi(expr) | StmtKind::Expr(expr) => {
                self.visit_expr(expr, active_measurement_sources);
            }
            StmtKind::Item(_) => {}
            StmtKind::Qubit(source, pat, _, block) => {
                let kind = match source {
                    qsc_hir::hir::QubitSource::Fresh => QubitKind::Fresh,
                    qsc_hir::hir::QubitSource::Dirty => QubitKind::Dirty,
                };
                let initial_state = match kind {
                    QubitKind::Fresh => QubitState::Clean,
                    QubitKind::Dirty => QubitState::Dirty,
                    QubitKind::Parameter => QubitState::Dirty,
                };
                let idents = collect_pat_idents(pat);
                for ident in idents {
                    self.step_counter += 1;
                    let qubit_id = QubitId {
                        node_id: ident.id,
                        name: ident.name.to_string(),
                        span: (ident.span.lo as usize, ident.span.hi as usize),
                    };
                    let trans = QubitTransition {
                        step: self.step_counter,
                        kind: TransitionKind::Allocation(kind),
                        span: (ident.span.lo as usize, ident.span.hi as usize),
                        details: format!("Allocated as {}", kind),
                    };
                    self.lifetimes.push(QubitLifetime {
                        qubit: qubit_id,
                        kind,
                        state: initial_state,
                        transitions: vec![trans],
                        has_leak: false,
                        missing_uncompute: false,
                        release_span: None,
                        gate_history: Vec::new(),
                    });
                }
                if let Some(b) = block {
                    self.visit_block(b, active_measurement_sources);
                }
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr, active_measurement_sources: Option<&HashSet<String>>) {
        match &expr.kind {
            ExprKind::Call(callee, arg) => {
                self.process_call(callee, arg, expr.span, active_measurement_sources);
            }
            ExprKind::Block(block) => {
                self.visit_block(block, active_measurement_sources);
            }
            ExprKind::If(cond, then_expr, else_expr) => {
                let cond_sources = self.extract_measurement_sources_from_expr(cond);
                let combined_sources: Option<HashSet<String>> = match (active_measurement_sources, cond_sources.is_empty()) {
                    (Some(prev), true) => Some(prev.clone()),
                    (Some(prev), false) => {
                        let mut merged = prev.clone();
                        merged.extend(cond_sources);
                        Some(merged)
                    }
                    (None, false) => Some(cond_sources),
                    (None, true) => None,
                };

                self.visit_expr(then_expr, combined_sources.as_ref());
                if let Some(e) = else_expr {
                    self.visit_expr(e, combined_sources.as_ref());
                }
            }
            ExprKind::While(_, block) => {
                self.visit_block(block, active_measurement_sources);
            }
            ExprKind::For(_, iter, block) => {
                self.visit_expr(iter, active_measurement_sources);
                self.visit_block(block, active_measurement_sources);
            }
            ExprKind::Conjugate(within_block, apply_block) => {
                let saved_states: Vec<(NodeId, QubitState)> = self
                    .lifetimes
                    .iter()
                    .map(|l| (l.qubit.node_id, l.state))
                    .collect();

                self.visit_block(within_block, active_measurement_sources);
                self.visit_block(apply_block, active_measurement_sources);

                for (id, prev_state) in saved_states {
                    if let Some(l) = self.lifetimes.iter_mut().find(|l| l.qubit.node_id == id) {
                        if prev_state == QubitState::Clean && l.state == QubitState::Dirty {
                            l.state = QubitState::Clean;
                        }
                    }
                }
            }
            ExprKind::Tuple(exprs) | ExprKind::Array(exprs) => {
                for e in exprs {
                    self.visit_expr(e, active_measurement_sources);
                }
            }
            ExprKind::Return(e) | ExprKind::UnOp(_, e) => {
                self.visit_expr(e, active_measurement_sources);
            }
            ExprKind::BinOp(_, e1, e2) | ExprKind::Assign(e1, e2) => {
                self.visit_expr(e1, active_measurement_sources);
                self.visit_expr(e2, active_measurement_sources);
            }
            _ => {}
        }
    }

    fn process_call(
        &mut self,
        callee: &Expr,
        arg: &Expr,
        call_span: Span,
        active_measurement_sources: Option<&HashSet<String>>,
    ) {
        let (callee_name, is_adjoint) = self.resolve_callee_name_and_adj(callee);
        let qubits = self.extract_qubit_refs(arg);

        if is_qubit_releaser(&callee_name) {
            for q_id in qubits {
                self.step_counter += 1;
                let step = self.step_counter;
                let rel_span = (call_span.lo as usize, call_span.hi as usize);
                if let Some(l) = self.find_lifetime_mut(q_id) {
                    l.release_span = Some(rel_span);
                    l.transitions.push(QubitTransition {
                        step,
                        kind: TransitionKind::Release,
                        span: rel_span,
                        details: format!("Released via {}", callee_name),
                    });
                    if l.kind == QubitKind::Fresh
                        && (l.state == QubitState::Dirty || l.state == QubitState::Measured)
                    {
                        l.has_leak = true;
                    }
                    l.state = QubitState::Released;
                }
            }
            return;
        }

        if is_reset(&callee_name) {
            for q_id in qubits {
                self.step_counter += 1;
                let step = self.step_counter;
                let span = (call_span.lo as usize, call_span.hi as usize);
                if let Some(l) = self.find_lifetime_mut(q_id) {
                    if is_measurement(&callee_name) {
                        l.transitions.push(QubitTransition {
                            step,
                            kind: TransitionKind::Measurement(callee_name.clone()),
                            span,
                            details: format!("Measured via {}", callee_name),
                        });
                    }
                    l.state = QubitState::Clean;
                    l.transitions.push(QubitTransition {
                        step,
                        kind: TransitionKind::Reset(callee_name.clone()),
                        span,
                        details: format!("Reset via {}", callee_name),
                    });
                }
            }
            return;
        }

        if is_measurement(&callee_name) {
            for q_id in qubits {
                self.step_counter += 1;
                let step = self.step_counter;
                let span = (call_span.lo as usize, call_span.hi as usize);
                if let Some(l) = self.find_lifetime_mut(q_id) {
                    l.state = QubitState::Measured;
                    l.transitions.push(QubitTransition {
                        step,
                        kind: TransitionKind::Measurement(callee_name.clone()),
                        span,
                        details: format!("Measured via {}", callee_name),
                    });
                }
            }
            return;
        }

        if !qubits.is_empty() {
            self.step_counter += 1;
            let step = self.step_counter;
            let span = (call_span.lo as usize, call_span.hi as usize);

            if let Some(sources) = active_measurement_sources {
                for q_id in &qubits {
                    let target_name = self.find_lifetime(*q_id).map(|l| l.qubit.name.clone());
                    if let Some(t_name) = target_name {
                        for source_name in sources {
                            if source_name != &t_name {
                                self.entanglements.push(EntanglementEvent {
                                    measurement_span: span,
                                    condition_span: span,
                                    action_span: span,
                                    source_qubit: source_name.clone(),
                                    target_qubit: t_name.clone(),
                                });
                            }
                        }
                    }
                }
            }

            let gate_app = GateApplication {
                gate_name: callee_name.clone(),
                qubits: qubits.clone(),
                is_adjoint,
                span,
                step,
            };

            for q_id in qubits {
                if let Some(l) = self.find_lifetime_mut(q_id) {
                    l.state = QubitState::Dirty;
                    l.gate_history.push(gate_app.clone());
                    l.transitions.push(QubitTransition {
                        step,
                        kind: TransitionKind::Gate(callee_name.clone()),
                        span,
                        details: format!("Applied gate {}", callee_name),
                    });
                }
            }
        }
    }

    fn as_call_info<'e>(&self, expr: &'e Expr) -> Option<(String, &'e Expr)> {
        if let ExprKind::Call(callee, arg) = &expr.kind {
            let (name, _) = self.resolve_callee_name_and_adj(callee);
            Some((name, arg.as_ref()))
        } else {
            None
        }
    }

    fn resolve_callee_name_and_adj(&self, callee: &Expr) -> (String, bool) {
        match &callee.kind {
            ExprKind::Var(Res::Item(item_id), _) => {
                let name = self.resolve_item_name(item_id).unwrap_or("unknown");
                (name.to_string(), false)
            }
            ExprKind::UnOp(UnOp::Functor(qsc_hir::hir::Functor::Adj), inner) => {
                let (name, _) = self.resolve_callee_name_and_adj(inner);
                (name, true)
            }
            ExprKind::UnOp(UnOp::Functor(qsc_hir::hir::Functor::Ctl), inner) => {
                let (name, adj) = self.resolve_callee_name_and_adj(inner);
                (format!("Controlled {}", name), adj)
            }
            ExprKind::Var(Res::Local(node_id), _) => {
                let name = format!("local_{}", node_id);
                (name, false)
            }
            _ => ("unknown".to_string(), false),
        }
    }

    fn resolve_item_name(&self, item_id: &ItemId) -> Option<&str> {
        let u = self.store.get(item_id.package).unwrap_or(self.unit);
        if let Some(item) = u.package.items.get(item_id.item) {
            if let ItemKind::Callable(decl) = &item.kind {
                return Some(&decl.name.name);
            }
        }
        None
    }

    fn extract_qubit_refs(&self, expr: &Expr) -> Vec<NodeId> {
        let mut refs = Vec::new();
        self.collect_qubit_refs_rec(expr, &mut refs);
        refs
    }

    fn collect_qubit_refs_rec(&self, expr: &Expr, refs: &mut Vec<NodeId>) {
        match &expr.kind {
            ExprKind::Var(Res::Local(node_id), _) => {
                let canonical = self.resolve_node_id(*node_id);
                if self.lifetimes.iter().any(|l| l.qubit.node_id == canonical) {
                    if !refs.contains(&canonical) {
                        refs.push(canonical);
                    }
                }
            }
            ExprKind::Tuple(items) | ExprKind::Array(items) => {
                for item in items {
                    self.collect_qubit_refs_rec(item, refs);
                }
            }
            ExprKind::Index(base, _) => {
                self.collect_qubit_refs_rec(base, refs);
            }
            ExprKind::UnOp(_, inner) => {
                self.collect_qubit_refs_rec(inner, refs);
            }
            _ => {}
        }
    }

    fn extract_measurement_sources_from_expr(&self, expr: &Expr) -> HashSet<String> {
        let mut sources = HashSet::new();
        self.collect_measurement_sources_rec(expr, &mut sources);
        sources
    }

    fn collect_measurement_sources_rec(&self, expr: &Expr, sources: &mut HashSet<String>) {
        match &expr.kind {
            ExprKind::Var(Res::Local(node_id), _) => {
                if let Some((q_name, _)) = self.measurement_results.get(node_id) {
                    sources.insert(q_name.clone());
                }
            }
            ExprKind::BinOp(_, e1, e2) => {
                self.collect_measurement_sources_rec(e1, sources);
                self.collect_measurement_sources_rec(e2, sources);
            }
            ExprKind::UnOp(_, e) => {
                self.collect_measurement_sources_rec(e, sources);
            }
            ExprKind::Tuple(items) | ExprKind::Array(items) => {
                for i in items {
                    self.collect_measurement_sources_rec(i, sources);
                }
            }
            _ => {}
        }
    }
}

fn collect_pat_idents(pat: &Pat) -> Vec<&Ident> {
    let mut idents = Vec::new();
    collect_pat_idents_rec(pat, &mut idents);
    idents
}

fn collect_pat_idents_rec<'p>(pat: &'p Pat, idents: &mut Vec<&'p Ident>) {
    match &pat.kind {
        PatKind::Bind(ident) => {
            idents.push(ident);
        }
        PatKind::Tuple(pats) => {
            for p in pats {
                collect_pat_idents_rec(p, idents);
            }
        }
        PatKind::Discard | PatKind::Err => {}
    }
}

pub fn is_qubit_allocator(name: &str) -> bool {
    name == "__quantum__rt__qubit_allocate"
        || name == "__quantum__rt__qubit_borrow"
        || name == "AllocateQubitArray"
        || name == "BorrowQubitArray"
        || name == "Qubit"
}

pub fn is_qubit_releaser(name: &str) -> bool {
    name == "__quantum__rt__qubit_release" || name == "ReleaseQubitArray"
}

pub fn is_reset(name: &str) -> bool {
    name == "Reset" || name == "ResetAll" || name == "MResetZ" || name == "MResetX"
}

pub fn is_measurement(name: &str) -> bool {
    name == "M" || name == "Measure" || name == "MResetZ" || name == "MResetX"
}
