table! {
    candles (base, quote, period, timestamp) {
        base -> Varchar,
        quote -> Varchar,
        period -> Int4,
        timestamp -> Timestamptz,
        high -> Nullable<Float4>,
        low -> Nullable<Float4>,
        open -> Nullable<Float4>,
        close -> Nullable<Float4>,
        average -> Nullable<Float4>,
        volume -> Nullable<Float4>,
    }
}

table! {
    shortlist (quote) {
        quote -> Varchar,
        timestamp -> Nullable<Timestamptz>,
        average -> Nullable<Float4>,
        confidence -> Nullable<Float4>,
    }
}

allow_tables_to_appear_in_same_query!(
    candles,
    shortlist,
);
