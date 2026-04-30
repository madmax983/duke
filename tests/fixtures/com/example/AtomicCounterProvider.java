package com.example;

import java.util.concurrent.atomic.AtomicLong;

public final class AtomicCounterProvider implements AtomicCounterService {
    private static final AtomicLong COUNTER = new AtomicLong();
    private static long lastConstructedCount;

    public AtomicCounterProvider() {
        lastConstructedCount = COUNTER.incrementAndGet();
    }

    @Override
    public long constructedCount() {
        return lastConstructedCount;
    }
}
