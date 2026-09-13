<script lang="ts">
  interface Props {
    width?: string;
    height?: string;
    rounded?: string;
  }

  let { width = '100%', height = '1rem', rounded = 'var(--radius-sm)' }: Props = $props();
</script>

<div 
  class="skeleton"
  style="width: {width}; height: {height}; border-radius: {rounded};"
></div>

<style>
  .skeleton {
    background: linear-gradient(
      90deg,
      var(--bg-subtle) 25%,
      var(--border-subtle) 50%,
      var(--bg-subtle) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-shimmer 1.5s ease-in-out infinite;
  }

  @keyframes skeleton-shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>