namespace TestGoodUncompute {
    operation CorrectUncompute(target : Qubit) : Unit {
        use anc = Qubit();
        CNOT(target, anc);
        X(anc);
        X(anc);
        CNOT(target, anc);
        Reset(anc);
    }
}

