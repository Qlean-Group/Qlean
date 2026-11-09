namespace TestLeak {
    operation LeakyOp() : Unit {
        use q = Qubit();
        X(q);
    }
}

