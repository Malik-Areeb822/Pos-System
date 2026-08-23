import { useState } from "react";
import { toast } from "sonner";
import { z } from "zod";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { supabase } from "@/integrations/supabase/client";

const schema = z.object({
  name: z.string().trim().min(2, "Please enter your name").max(100),
  phone: z.string().trim().min(7, "Please enter a valid phone number").max(30),
  email: z.string().trim().email("Invalid email").max(255).optional().or(z.literal("")),
  company: z.string().trim().max(120).optional().or(z.literal("")),
  message: z.string().trim().min(5, "Tell us what you need").max(1500),
});

export function InquiryForm({
  inquiryType = "general",
  showCompany = false,
  submitLabel = "Send Inquiry",
  presetMessage = "",
}: {
  inquiryType?: string;
  showCompany?: boolean;
  submitLabel?: string;
  presetMessage?: string;
}) {
  const [values, setValues] = useState({
    name: "",
    phone: "",
    email: "",
    company: "",
    message: presetMessage,
  });
  const [loading, setLoading] = useState(false);

  const set = (key: keyof typeof values) => (e: { target: { value: string } }) =>
    setValues((v) => ({ ...v, [key]: e.target.value }));

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const parsed = schema.safeParse(values);
    if (!parsed.success) {
      toast.error(parsed.error.issues[0]?.message ?? "Please check the form");
      return;
    }
    setLoading(true);
    const { error } = await supabase.from("inquiries").insert({
      name: parsed.data.name,
      phone: parsed.data.phone,
      email: parsed.data.email || null,
      company: parsed.data.company || null,
      inquiry_type: inquiryType,
      message: parsed.data.message,
    });
    setLoading(false);
    if (error) {
      toast.error("Could not send your inquiry. Please call us instead.");
      return;
    }
    toast.success("Thank you — our team will contact you shortly.");
    setValues({ name: "", phone: "", email: "", company: "", message: "" });
  }

  return (
    <form onSubmit={onSubmit} className="space-y-5">
      <div className="grid gap-5 sm:grid-cols-2">
        <div className="space-y-2">
          <Label htmlFor="name">Name</Label>
          <Input id="name" value={values.name} onChange={set("name")} maxLength={100} />
        </div>
        <div className="space-y-2">
          <Label htmlFor="phone">Phone / WhatsApp</Label>
          <Input id="phone" value={values.phone} onChange={set("phone")} maxLength={30} />
        </div>
        <div className="space-y-2">
          <Label htmlFor="email">Email (optional)</Label>
          <Input id="email" value={values.email} onChange={set("email")} maxLength={255} />
        </div>
        {showCompany && (
          <div className="space-y-2">
            <Label htmlFor="company">Company / Site</Label>
            <Input id="company" value={values.company} onChange={set("company")} maxLength={120} />
          </div>
        )}
      </div>
      <div className="space-y-2">
        <Label htmlFor="message">What do you need?</Label>
        <Textarea
          id="message"
          rows={5}
          value={values.message}
          onChange={set("message")}
          maxLength={1500}
          placeholder="Product, quantity, size/finish and delivery location"
        />
      </div>
      <Button type="submit" variant="brass" size="xl" disabled={loading}>
        {loading ? "Sending…" : submitLabel}
      </Button>
    </form>
  );
}
