import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Modal } from '../Modal'
import { Input, Textarea } from '../Input'
import { Select } from '../Select'
import { Card, CardHeader, CardContent, CardFooter } from '../Card'

describe('Modal', () => {
  it('renders when isOpen is true', () => {
    render(
      <Modal isOpen={true} onClose={() => {}} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    expect(screen.getByText('Test Modal')).toBeInTheDocument()
    expect(screen.getByText('Modal content')).toBeInTheDocument()
  })

  it('does not render when isOpen is false', () => {
    render(
      <Modal isOpen={false} onClose={() => {}} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    expect(screen.queryByText('Test Modal')).not.toBeInTheDocument()
  })

  it('calls onClose when close button is clicked', async () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen={true} onClose={onClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    await userEvent.click(screen.getByLabelText('إغلاق'))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('calls onClose when overlay is clicked', async () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen={true} onClose={onClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    await userEvent.click(screen.getByLabelText('إغلاق'))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('does not close on overlay click when closeOnOverlayClick is false', async () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen={true} onClose={onClose} title="Test Modal" closeOnOverlayClick={false}>
        <p>Modal content</p>
      </Modal>
    )
    await userEvent.click(screen.getByLabelText('إغلاق'))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('has correct ARIA attributes', () => {
    render(
      <Modal isOpen={true} onClose={() => {}} title="Test Modal" description="Test description">
        <p>Modal content</p>
      </Modal>
    )
    expect(screen.getByRole('dialog')).toHaveAttribute('aria-modal', 'true')
    expect(screen.getByText('Test Modal')).toHaveAttribute('id', 'modal-title')
    expect(screen.getByText('Test description')).toHaveAttribute('id', 'modal-description')
  })

  it('calls onClose on Escape key', () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen={true} onClose={onClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('ignores Escape when closeOnEscape is false and other keys', () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen={true} onClose={onClose} title="Test Modal" closeOnEscape={false}>
        <p>Modal content</p>
      </Modal>
    )
    fireEvent.keyDown(document, { key: 'Escape' })
    fireEvent.keyDown(document, { key: 'Enter' })
    expect(onClose).not.toHaveBeenCalled()
  })

  it('renders bare dialog without title or close button', () => {
    render(
      <Modal isOpen={true} onClose={() => {}} showCloseButton={false}>
        <p>bare content</p>
      </Modal>
    )
    expect(screen.getByText('bare content')).toBeInTheDocument()
    expect(screen.queryByLabelText('إغلاق')).not.toBeInTheDocument()
    expect(screen.getByRole('dialog')).not.toHaveAttribute('aria-labelledby')
    expect(screen.getByRole('dialog')).not.toHaveAttribute('aria-describedby')
  })
})

describe('Input', () => {
  it('renders with placeholder', () => {
    render(<Input placeholder="Enter text..." />)
    expect(screen.getByPlaceholderText('Enter text...')).toBeInTheDocument()
  })

  it('renders with label', () => {
    render(<Input label="Username" id="username" />)
    expect(screen.getByLabelText('Username')).toBeInTheDocument()
  })

  it('shows error state', () => {
    render(<Input label="Email" error="Invalid email" />)
    expect(screen.getByText('Invalid email')).toBeInTheDocument()
    expect(screen.getByLabelText('Email')).toHaveAttribute('aria-invalid', 'true')
  })

  it('shows helper text when no error', () => {
    render(<Input label="Password" helperText="Must be 8+ characters" />)
    expect(screen.getByText('Must be 8+ characters')).toBeInTheDocument()
  })

  it('hides helper text when error is present', () => {
    render(<Input label="Password" error="Too short" helperText="Must be 8+ characters" />)
    expect(screen.queryByText('Must be 8+ characters')).not.toBeInTheDocument()
    expect(screen.getByText('Too short')).toBeInTheDocument()
  })

  it('handles disabled state', () => {
    render(<Input label="Disabled" disabled />)
    expect(screen.getByLabelText('Disabled')).toBeDisabled()
  })

  it('forwards ref correctly', () => {
    const ref = { current: null as HTMLInputElement | null }
    render(<Input ref={ref as any} label="With Ref" />)
    expect(ref.current).toBeInstanceOf(HTMLInputElement)
  })
})

describe('Textarea', () => {
  it('renders with placeholder', () => {
    render(<Textarea placeholder="Enter message..." />)
    expect(screen.getByPlaceholderText('Enter message...')).toBeInTheDocument()
  })

  it('renders with label', () => {
    render(<Textarea label="Message" id="message" />)
    expect(screen.getByLabelText('Message')).toBeInTheDocument()
  })

  it('shows error state', () => {
    render(<Textarea label="Bio" error="Too long" />)
    expect(screen.getByText('Too long')).toBeInTheDocument()
  })

  it('has resize-y class', () => {
    render(<Textarea placeholder="Enter text" />)
    expect(screen.getByPlaceholderText('Enter text')).toHaveClass('resize-y')
  })

  it('shows helper text when no error', () => {
    render(<Textarea helperText="Type here" />)
    expect(screen.getByText('Type here')).toBeInTheDocument()
  })
})

describe('Select', () => {
  const options = [
    { value: 'ar', label: 'العربية' },
    { value: 'en', label: 'English' },
  ]

  it('renders with options', () => {
    render(<Select label="Language" options={options} />)
    expect(screen.getByLabelText('Language')).toBeInTheDocument()
    expect(screen.getByText('العربية')).toBeInTheDocument()
    expect(screen.getByText('English')).toBeInTheDocument()
  })

  it('calls onChange when selection changes', async () => {
    const onChange = vi.fn()
    render(<Select label="Language" options={options} onChange={onChange} />)
    await userEvent.selectOptions(screen.getByLabelText('Language'), 'en')
    expect(onChange).toHaveBeenCalledTimes(1)
  })

  it('shows error state', () => {
    render(<Select label="Language" options={options} error="Required" />)
    expect(screen.getByText('Required')).toBeInTheDocument()
  })

  it('renders with placeholder', () => {
    render(<Select label="Language" options={options} placeholder="Select language" />)
    expect(screen.getByText('Select language')).toBeInTheDocument()
  })
})

describe('Card', () => {
  it('renders children', () => {
    render(<Card>Card content</Card>)
    expect(screen.getByText('Card content')).toBeInTheDocument()
  })

  it('renders with hover effect', () => {
    render(<Card hover>Hoverable card</Card>)
    const card = screen.getByText('Hoverable card').closest('.rounded-xl')
    expect(card).toHaveClass('hover:shadow-md')
  })

  it('renders with different padding', () => {
    const { rerender } = render(<Card padding="none">No padding</Card>)
    let card = screen.getByText('No padding').closest('.rounded-xl')
    expect(card).not.toHaveClass('p-6')

    rerender(<Card padding="sm">Small padding</Card>)
    card = screen.getByText('Small padding').closest('.rounded-xl')
    expect(card).toHaveClass('p-4')

    rerender(<Card padding="md">Medium padding</Card>)
    card = screen.getByText('Medium padding').closest('.rounded-xl')
    expect(card).toHaveClass('p-6')

    rerender(<Card padding="lg">Large padding</Card>)
    card = screen.getByText('Large padding').closest('.rounded-xl')
    expect(card).toHaveClass('p-8')
  })

  it('renders CardHeader with title', () => {
    render(<CardHeader title="Card Title" />)
    expect(screen.getByText('Card Title')).toBeInTheDocument()
  })

  it('renders CardHeader with description', () => {
    render(<CardHeader title="Title" description="Description" />)
    expect(screen.getByText('Description')).toBeInTheDocument()
  })

  it('renders CardHeader with action', () => {
    render(<CardHeader title="Title" action={<button>Action</button>} />)
    expect(screen.getByRole('button', { name: 'Action' })).toBeInTheDocument()
  })

  it('renders CardHeader with icon', () => {
    render(<CardHeader title="Title" icon={<span data-testid="hicon">I</span>} />)
    expect(screen.getByTestId('hicon')).toBeInTheDocument()
  })

  it('renders CardContent children', () => {
    render(<CardContent>Content here</CardContent>)
    expect(screen.getByText('Content here')).toBeInTheDocument()
  })

  it('renders CardFooter children', () => {
    render(<CardFooter>Footer content</CardFooter>)
    expect(screen.getByText('Footer content')).toBeInTheDocument()
  })
})
