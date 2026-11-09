namespace CorpusSuperdense {
    operation SuperdenseCoding() : Unit {
        use (msg, target) = (Qubit(), Qubit());
        H(msg);
        CNOT(msg, target);
        X(msg);
        CNOT(msg, target);
        H(msg);
        Reset(msg);
        Reset(target);
    }
}

