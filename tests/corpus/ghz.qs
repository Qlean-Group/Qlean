namespace CorpusGhz {
    operation PrepareGhz() : Unit {
        use (q0, q1, q2) = (Qubit(), Qubit(), Qubit());
        H(q0);
        CNOT(q0, q1);
        CNOT(q1, q2);
        Reset(q0);
        Reset(q1);
        Reset(q2);
    }
}

