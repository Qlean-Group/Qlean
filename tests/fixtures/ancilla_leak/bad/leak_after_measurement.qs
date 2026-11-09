namespace TestLeakMeasurement {
    operation LeakyMeasurement() : Unit {
        use q = Qubit();
        let m = M(q);
    }
}

