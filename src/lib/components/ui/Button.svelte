<script lang="ts">
  import { type Snippet } from 'svelte';

  type Props = {
    variant?: 'default' | 'secondary' | 'ghost' | 'destructive' | 'outline';
    size?: 'sm' | 'default' | 'icon';
    class?: string;
    children?: Snippet;
    onclick?: (e: MouseEvent) => void;
    disabled?: boolean;
    title?: string;
    type?: 'button' | 'submit' | 'reset';
  };

  let {
    variant = 'default',
    size = 'default',
    class: className = '',
    children,
    onclick,
    disabled = false,
    title,
    type = 'button',
    ...rest
  }: Props = $props();

  const variants = {
    default: 'bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm',
    secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
    ghost: 'hover:bg-accent hover:text-accent-foreground',
    destructive: 'bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-sm',
    outline: 'border border-input bg-background hover:bg-accent hover:text-accent-foreground'
  };

  const sizes = {
    default: 'h-9 px-4 py-2',
    sm: 'h-8 rounded-md px-3 text-xs',
    icon: 'h-9 w-9'
  };
</script>

<button
  class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 {variants[variant]} {sizes[size]} {className}"
  {onclick}
  {disabled}
  {title}
  {type}
  {...rest}
>
  {@render children?.()}
</button>
