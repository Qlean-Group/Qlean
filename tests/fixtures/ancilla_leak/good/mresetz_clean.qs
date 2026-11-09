namespace TestCleanMReset {
    operation CleanWithMReset() : Unit {
        use q = Qubit();
        H(q);
        let res = MResetZ(q);
    }
}

