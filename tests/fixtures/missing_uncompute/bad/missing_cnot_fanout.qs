namespace TestBadUncompute {
    operation MissingUncomputeOp(target : Qubit) : Unit {
        use anc = Qubit();
        CNOT(target, anc);
        Reset(anc);
    }
}

