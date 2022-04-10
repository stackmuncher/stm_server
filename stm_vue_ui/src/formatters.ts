// Formats a numeric value like 1000 into 1,000.
export function numFmt(num: number) {
  return num.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}
