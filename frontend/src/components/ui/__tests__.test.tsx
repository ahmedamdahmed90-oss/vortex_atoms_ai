import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Button } from './Button'
import { Badge } from './Badge'
import { Spinner, LoadingOverlay } from './Spinner'

describe('Button', () => {
  it('renders with default props', () => {
    render(<Button>Click me</Button>)
    expect(screen.getByRole('button')).toHaveTextContent('Click me')
  })

  it('renders different variants', () => {
    const { rerender } = render(<Button variant="primary">Primary</Button>)
    expect(screen.getByRole('button')).toHaveClass('bg-primary-600')

    rerender(<Button variant="secondary">Secondary</Button>)
    expect(screen.getByRole('button')).toHaveClass('bg-gray-100')

    rerender(<Button variant="danger">Danger</Button>)
    expect(screen.getByRole('button')).toHaveClass('bg-red-600')
  })

  it('shows loading state', () => {
    render(<Button loading>Submit</Button>)
    expect(screen.getByRole('button')).toBeDisabled()
    expect(screen.getByRole('button')).toContainHTML('animate-spin')
  })

  it('disables when disabled prop is true', () => {
    render(<Button disabled>Disabled</Button>)
    expect(screen.getByRole('button')).toBeDisabled()
  })

  it('handles click events', () => {
    const handleClick = vi.fn()
    render(<Button onClick={handleClick}>Click</Button>)
    fireEvent.click(screen.getByRole('button'))
    expect(handleClick).toHaveBeenCalledTimes(1)
  })

  it('renders with left icon', () => {
    render(<Button leftIcon={<span data-testid="icon">🔥</span>}>With Icon</Button>)
    expect(screen.getByTestId('icon')).toBeInTheDocument()
  })

  it('renders with right icon', () => {
    render(<Button rightIcon={<span data-testid="ricon">®</span>}>With RIcon</Button>)
    expect(screen.getByTestId('ricon')).toBeInTheDocument()
  })
})

describe('Badge', () => {
  it('renders with default variant', () => {
    render(<Badge>Default</Badge>)
    expect(screen.getByText('Default')).toBeInTheDocument()
  })

  it('renders different variants', () => {
    const { rerender } = render(<Badge variant="success">Success</Badge>)
    expect(screen.getByText('Success')).toHaveClass('bg-green-100')

    rerender(<Badge variant="error">Error</Badge>)
    expect(screen.getByText('Error')).toHaveClass('bg-red-100')

    rerender(<Badge variant="warning">Warning</Badge>)
    expect(screen.getByText('Warning')).toHaveClass('bg-yellow-100')

    rerender(<Badge variant="info">Info</Badge>)
    expect(screen.getByText('Info')).toHaveClass('bg-blue-100')
  })

  it('renders different sizes', () => {
    const { rerender } = render(<Badge size="sm">Small</Badge>)
    expect(screen.getByText('Small')).toHaveClass('text-xs')

    rerender(<Badge size="md">Medium</Badge>)
    expect(screen.getByText('Medium')).toHaveClass('text-sm')

    rerender(<Badge size="lg">Large</Badge>)
    expect(screen.getByText('Large')).toHaveClass('text-base')
  })
})

describe('Spinner', () => {
  it('renders with default size', () => {
    render(<Spinner />)
    const spinner = document.querySelector('.animate-spin')
    expect(spinner).toBeInTheDocument()
  })

it('renders different sizes', () => {
    const { rerender } = render(<Spinner size="sm" />)
    let spinner = document.querySelector('.animate-spin')
    expect(spinner).toHaveClass('h-4')

    rerender(<Spinner size="md" />)
    spinner = document.querySelector('.animate-spin')
    expect(spinner).toHaveClass('h-6')

    rerender(<Spinner size="lg" />)
    spinner = document.querySelector('.animate-spin')
    expect(spinner).toHaveClass('h-8')
  })

  it('has aria-hidden for accessibility', () => {
    render(<Spinner />)
    const spinner = document.querySelector('.animate-spin')
    expect(spinner).toHaveAttribute('aria-hidden', 'true')
  })
})

describe('LoadingOverlay', () => {
  it('renders children without overlay when idle', () => {
    const { container } = render(<LoadingOverlay isLoading={false}><div>content</div></LoadingOverlay>)
    expect(screen.getByText('content')).toBeInTheDocument()
    expect(container.querySelector('.absolute.inset-0')).not.toBeInTheDocument()
  })

  it('shows overlay with message when loading', () => {
    render(<LoadingOverlay isLoading message="يرجى الانتظار"><div>content</div></LoadingOverlay>)
    expect(screen.getByText('يرجى الانتظار')).toBeInTheDocument()
    expect(screen.getByText('content')).toBeInTheDocument()
  })

  it('shows overlay without message text when none given', () => {
    const { container } = render(<LoadingOverlay isLoading><div>content</div></LoadingOverlay>)
    expect(container.querySelector('.absolute.inset-0')).toBeInTheDocument()
  })
})
