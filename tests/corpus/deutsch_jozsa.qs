namespace CorpusDeutschJozsa {
    operation RunDeutschJozsa() : Unit {
        use (x, y) = (Qubit(), Qubit());
        X(y);
        H(x);
        H(y);
        CNOT(x, y);
        H(x);
        let res = MResetZ(x);
        Reset(y);
    }
}

