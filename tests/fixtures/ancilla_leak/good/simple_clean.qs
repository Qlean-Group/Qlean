namespace TestClean {
    operation CleanOp() : Unit {
        use q = Qubit();
        H(q);
        Reset(q);
    }
}

