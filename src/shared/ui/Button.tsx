import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { classes } from './classes';
import styles from './Button.module.css';

type Tone = 'default' | 'primary' | 'ghost';

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'className'> {
  tone?: Tone;
  children: ReactNode;
}

/**
 * The one button in the application.
 *
 * Three tones and no sizes. A second size is a decision to make on every screen
 * and a way for two screens to disagree, and nothing so far has needed one.
 */
export function Button({ tone = 'default', type = 'button', ...props }: ButtonProps) {
  return <button type={type} className={classes(styles.button, styles[tone])} {...props} />;
}
