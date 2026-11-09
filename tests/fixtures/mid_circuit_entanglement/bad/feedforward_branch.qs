namespace TestFeedforward {
    operation FeedbackOp() : Unit {
        use q1 = Qubit();
        use q2 = Qubit();
        let res = M(q1);
        if res == One {
            X(q2);
        }
        Reset(q1);
        Reset(q2);
    }
}

