namespace CorpusBernsteinVazirani {
    operation BernsteinVazirani() : Unit {
        use (q0, q1, anc) = (Qubit(), Qubit(), Qubit());
        X(anc);
        H(q0);
        H(q1);
        H(anc);
        CNOT(q0, anc);
        CNOT(q1, anc);
        H(q0);
        H(q1);
        let r0 = MResetZ(q0);
        let r1 = MResetZ(q1);
        Reset(anc);
    }
}

