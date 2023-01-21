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
        timestamp -> Timestamptz,
        average -> Float4,
        target -> Float4,
        confidence -> Float4,
    }
}

table! {
    trades (id) {
        id -> Int4,
        base -> Varchar,
        quote -> Varchar,
        open_at -> Timestamptz,
        close_at -> Nullable<Timestamptz>,
        updated_at -> Timestamptz,
        open_average -> Float4,
        target -> Float4,
        open -> Nullable<Float4>,
        close -> Nullable<Float4>,
        highest_bid -> Nullable<Float4>,
    }
}

allow_tables_to_appear_in_same_query!(
    candles,
    shortlist,
    trades,
);
