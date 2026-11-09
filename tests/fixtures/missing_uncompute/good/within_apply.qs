namespace TestWithinApply {
    operation WithinApplyUncompute(target : Qubit) : Unit {
        use anc = Qubit();
        within {
            CNOT(target, anc);
        } apply {
            Z(target);
        }
        Reset(anc);
    }
}

