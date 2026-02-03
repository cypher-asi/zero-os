import styles from './AuthLink.module.css';

interface AuthLinkProps {
  text: string;
  linkText: string;
  onClick: () => void;
  disabled?: boolean;
}

/**
 * AuthLink - Shared link component for switching between login/registration
 */
export function AuthLink({ text, linkText, onClick, disabled }: AuthLinkProps) {
  return (
    <div className={styles.authLink}>
      <span className={styles.text}>{text}</span>{' '}
      <button
        type="button"
        onClick={onClick}
        disabled={disabled}
        className={styles.linkButton}
      >
        {linkText}
      </button>
    </div>
  );
}
