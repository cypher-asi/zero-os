import styles from './CurrencyDisplay.module.css';

/**
 * Format a number with K/M suffixes, always showing 4 significant digits.
 * - Up to 9,999: show full number (e.g., "1234", "9999")
 * - 10,000+: show with K suffix (e.g., "12.34K", "123.4K", "999.9K")
 * - 1,000,000+: show with M suffix (e.g., "1.234M", "12.34M", "123.4M")
 */
function formatCompactAmount(amount: number): string {
  if (amount >= 1_000_000) {
    const millions = amount / 1_000_000;
    if (millions >= 100) return `${millions.toFixed(1)} M`;
    if (millions >= 10) return `${millions.toFixed(2)} M`;
    return `${millions.toFixed(3)} M`;
  }
  if (amount >= 10_000) {
    const thousands = amount / 1_000;
    if (thousands >= 100) return `${thousands.toFixed(1)} K`;
    return `${thousands.toFixed(2)} K`;
  }
  return amount.toString();
}

export function CurrencyDisplay() {
  // Mock value - will be replaced with real data later
  const amount = 1234567;
  const formattedAmount = formatCompactAmount(amount);
  const fullAmount = amount.toLocaleString('en-US', { maximumFractionDigits: 0 });

  return (
    <div className={styles.currencyDisplay} title={`${fullAmount} Z`}>
      <span className={styles.amount}>{formattedAmount}</span>
      <span className={styles.currency}>Z</span>
    </div>
  );
}
