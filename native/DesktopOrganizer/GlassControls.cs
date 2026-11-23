using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Threading;

namespace MacUI.Desktop.Native
{
    internal static class GlassControls
    {
        private static readonly ResourceDictionary Resources = (ResourceDictionary)XamlReader.Parse(@"
<ResourceDictionary xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:local='clr-namespace:MacUI.Desktop.Native;assembly=macui-desktop'>
  <Style x:Key='GlassScrollBar' TargetType='{x:Type ScrollBar}'>
    <Setter Property='Width' Value='12'/><Setter Property='Background' Value='Transparent'/>
    <Setter Property='Template'><Setter.Value><ControlTemplate TargetType='{x:Type ScrollBar}'>
      <Grid Background='Transparent' Margin='4,4'>
        <Track x:Name='PART_Track' IsDirectionReversed='True' Orientation='Vertical'>
          <Track.DecreaseRepeatButton><RepeatButton Command='ScrollBar.PageUpCommand' Focusable='False'><RepeatButton.Template><ControlTemplate><Border Background='Transparent'/></ControlTemplate></RepeatButton.Template></RepeatButton></Track.DecreaseRepeatButton>
          <Track.Thumb><Thumb MinHeight='28' Background='#65FFFFFF'><Thumb.Template><ControlTemplate TargetType='{x:Type Thumb}'>
            <Border CornerRadius='2' Background='{TemplateBinding Background}' BorderBrush='#90FFFFFF' BorderThickness='0.5'/>
            <ControlTemplate.Triggers><Trigger Property='IsMouseOver' Value='True'><Setter Property='Background' Value='#A0FFFFFF'/></Trigger><Trigger Property='IsDragging' Value='True'><Setter Property='Background' Value='#D0FFFFFF'/></Trigger></ControlTemplate.Triggers>
          </ControlTemplate></Thumb.Template></Thumb></Track.Thumb>
          <Track.IncreaseRepeatButton><RepeatButton Command='ScrollBar.PageDownCommand' Focusable='False'><RepeatButton.Template><ControlTemplate><Border Background='Transparent'/></ControlTemplate></RepeatButton.Template></RepeatButton></Track.IncreaseRepeatButton>
        </Track>
      </Grid>
    </ControlTemplate></Setter.Value></Setter>
  </Style>
  <Style x:Key='GlassScrollViewer' TargetType='{x:Type ScrollViewer}'>
    <Setter Property='Template'><Setter.Value><ControlTemplate TargetType='{x:Type ScrollViewer}'>
      <Grid Background='{TemplateBinding Background}'>
        <ScrollContentPresenter x:Name='PART_ScrollContentPresenter' CanContentScroll='{TemplateBinding CanContentScroll}' Content='{TemplateBinding Content}' ContentTemplate='{TemplateBinding ContentTemplate}'/>
        <local:GlassScrollBar x:Name='PART_VerticalScrollBar' Style='{StaticResource GlassScrollBar}' HorizontalAlignment='Right' Orientation='Vertical' Maximum='{TemplateBinding ScrollableHeight}' ViewportSize='{TemplateBinding ViewportHeight}' Value='{Binding VerticalOffset, RelativeSource={RelativeSource TemplatedParent}, Mode=OneWay}' Visibility='{TemplateBinding ComputedVerticalScrollBarVisibility}'/>
      </Grid>
    </ControlTemplate></Setter.Value></Setter>
  </Style>
  <Style TargetType='{x:Type TextBox}'>
    <Setter Property='BorderBrush' Value='#80FFFFFF'/><Setter Property='BorderThickness' Value='1'/>
    <Setter Property='Template'><Setter.Value><ControlTemplate TargetType='{x:Type TextBox}'>
      <Border CornerRadius='9' Background='{TemplateBinding Background}' BorderBrush='{TemplateBinding BorderBrush}' BorderThickness='{TemplateBinding BorderThickness}' Padding='{TemplateBinding Padding}'>
        <ScrollViewer x:Name='PART_ContentHost'/>
      </Border>
    </ControlTemplate></Setter.Value></Setter>
  </Style>
</ResourceDictionary>".Replace("assembly=macui-desktop", "assembly=" + typeof(GlassControls).Assembly.GetName().Name));
        internal static void Apply(FrameworkElement element) { element.Resources.MergedDictionaries.Add(Resources); }
        internal static Style ButtonFocus(bool compact)
        {
            var border = new FrameworkElementFactory(typeof(Border));
            border.SetValue(Border.CornerRadiusProperty, new CornerRadius(compact ? 15 : 16));
            border.SetValue(Border.BorderThicknessProperty, new Thickness(2));
            border.SetValue(Border.BorderBrushProperty, Glass.Accent);
            var style = new Style(typeof(Control));
            style.Setters.Add(new Setter(Control.TemplateProperty, new ControlTemplate(typeof(Control)) { VisualTree = border }));
            return style;
        }
        internal static Brush Fill(bool contrast, int transparency = 88)
        {
            double opacity = (100 - transparency) / 100.0;
            if (contrast) opacity = Math.Max(0.38, opacity);
            return Glass.Brush((byte)(255 * opacity), 236, 245, 251);
        }
        internal static Border Frame(UIElement content, bool contrast, int transparency = 88)
        {
            var edge = new LinearGradientBrush(Color.FromArgb(210, 255, 255, 255), Color.FromArgb(40, 255, 255, 255), 85);
            var frame = new Border { CornerRadius = new CornerRadius(22), Background = Fill(contrast, transparency), BorderBrush = edge, BorderThickness = new Thickness(1), Child = content };
            frame.SizeChanged += delegate { frame.Clip = new RectangleGeometry(new Rect(0, 0, frame.ActualWidth, frame.ActualHeight), 22, 22); };
            return frame;
        }
    }

    public sealed class GlassScrollBar : ScrollBar
    {
        private readonly DispatcherTimer idle;
        private bool dragging;
        public GlassScrollBar()
        {
            Opacity = 0;
            idle = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(1100) };
            idle.Tick += delegate {
                idle.Stop();
                if (!dragging) BeginAnimation(OpacityProperty, new DoubleAnimation(0, TimeSpan.FromMilliseconds(220)));
            };
            ValueChanged += delegate { Reveal(); };
            MouseEnter += delegate { Reveal(); };
            PreviewMouseDown += delegate { Reveal(); };
            AddHandler(Thumb.DragStartedEvent, new DragStartedEventHandler(delegate { dragging = true; Reveal(); }));
            AddHandler(Thumb.DragCompletedEvent, new DragCompletedEventHandler(delegate { dragging = false; Reveal(); }));
            Unloaded += delegate { idle.Stop(); };
        }
        internal void Reveal()
        {
            BeginAnimation(OpacityProperty, null); Opacity = 1;
            idle.Stop(); if (!dragging) idle.Start();
        }
    }
}
