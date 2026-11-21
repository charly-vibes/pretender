# Fixture: R ships zero type captures (add-data-model-checks §6.6/D2);
# the data-model checks must stay silent for it (no excessive_fields or
# primitive_field findings, no diagnostics).
greet <- function(name) {
  paste("hello,", name)
}

add_counts <- function(a, b) {
  a + b
}
