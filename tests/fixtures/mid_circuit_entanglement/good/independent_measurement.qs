namespace TestIndependentMeasurement {
    operation MeasureWithoutFeedback() : Result {
        use q = Qubit();
        let r = MResetZ(q);
        return r;
    }
}

