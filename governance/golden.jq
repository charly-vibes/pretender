[.data.files[]
  | {path,
     units: ([((.units // [])
        | .[]
        | {name,
           metrics,
           violations: ([((.violations // [])[])] | sort_by(.metric))})]
        | sort_by(.name))}]
| sort_by(.path)
| {schema: "golden.v1", files: .}
