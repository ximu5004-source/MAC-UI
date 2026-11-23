using System;
using System.Collections.Generic;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Threading;

namespace MacUI.Desktop.Native
{
    internal sealed class GlassContextMenu : ContextMenu
    {
        private readonly DesktopSession session;
        private readonly Dictionary<Border, NativeGlass> surfaces = new Dictionary<Border, NativeGlass>();
        private static readonly ResourceDictionary MenuResources = (ResourceDictionary)XamlReader.Parse(@"
<ResourceDictionary xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Style TargetType='{x:Type ContextMenu}'>
    <Setter Property='OverridesDefaultStyle' Value='True'/><Setter Property='HasDropShadow' Value='False'/>
    <Setter Property='Template'><Setter.Value><ControlTemplate TargetType='{x:Type ContextMenu}'>
      <Border x:Name='MenuSurface' CornerRadius='22' BorderThickness='1' BorderBrush='#90FFFFFF' Background='{TemplateBinding Background}' Padding='7'>
        <ScrollViewer VerticalScrollBarVisibility='Auto' HorizontalScrollBarVisibility='Disabled' CanContentScroll='True'><ItemsPresenter KeyboardNavigation.DirectionalNavigation='Cycle'/></ScrollViewer>
      </Border>
    </ControlTemplate></Setter.Value></Setter>
  </Style>
  <Style TargetType='{x:Type Separator}'><Setter Property='Template'><Setter.Value><ControlTemplate TargetType='{x:Type Separator}'><Border Height='1' Margin='12,5' Background='#35FFFFFF'/></ControlTemplate></Setter.Value></Setter></Style>
  <Style TargetType='{x:Type MenuItem}'>
    <Setter Property='OverridesDefaultStyle' Value='True'/><Setter Property='Background' Value='Transparent'/><Setter Property='MinWidth' Value='220'/>
    <Setter Property='Template'><Setter.Value><ControlTemplate TargetType='{x:Type MenuItem}'>
      <Grid>
        <Border x:Name='Row' CornerRadius='10' Background='{TemplateBinding Background}' Padding='8,7' Margin='0,1'>
          <Grid><Grid.ColumnDefinitions><ColumnDefinition Width='20'/><ColumnDefinition Width='*'/><ColumnDefinition Width='18'/></Grid.ColumnDefinitions>
            <TextBlock x:Name='Check' Text='✓' Visibility='Hidden' VerticalAlignment='Center'/>
            <ContentPresenter Grid.Column='1' ContentSource='Header' RecognizesAccessKey='True' VerticalAlignment='Center'/>
            <TextBlock x:Name='Arrow' Grid.Column='2' Text='›' FontSize='17' Visibility='Hidden' HorizontalAlignment='Right' VerticalAlignment='Center'/>
          </Grid>
        </Border>
        <Popup x:Name='PART_Popup' AllowsTransparency='True' Focusable='False' IsOpen='{Binding IsSubmenuOpen, RelativeSource={RelativeSource TemplatedParent}}' Placement='Right' HorizontalOffset='2' PopupAnimation='None'>
          <Border x:Name='SubmenuSurface' CornerRadius='22' BorderThickness='1' BorderBrush='#90FFFFFF' Background='#40EAF3FE' Padding='7'>
            <ScrollViewer VerticalScrollBarVisibility='Auto' HorizontalScrollBarVisibility='Disabled'><ItemsPresenter KeyboardNavigation.DirectionalNavigation='Cycle'/></ScrollViewer>
          </Border>
        </Popup>
      </Grid>
      <ControlTemplate.Triggers>
        <Trigger Property='IsHighlighted' Value='True'><Setter TargetName='Row' Property='Background' Value='#50FFFFFF'/></Trigger>
        <Trigger Property='IsKeyboardFocusWithin' Value='True'><Setter TargetName='Row' Property='Background' Value='#50FFFFFF'/></Trigger>
        <Trigger Property='IsChecked' Value='True'><Setter TargetName='Check' Property='Visibility' Value='Visible'/></Trigger>
        <Trigger Property='HasItems' Value='True'><Setter TargetName='Arrow' Property='Visibility' Value='Visible'/></Trigger>
        <Trigger Property='IsEnabled' Value='False'><Setter TargetName='Row' Property='Opacity' Value='0.4'/></Trigger>
      </ControlTemplate.Triggers>
    </ControlTemplate></Setter.Value></Setter>
  </Style>
</ResourceDictionary>");
        internal GlassContextMenu(DesktopSession session)
        {
            this.session = session;
            Resources.MergedDictionaries.Add(MenuResources);
            Style = (Style)MenuResources[typeof(ContextMenu)];
            Foreground = Glass.Ink; FontSize = 13; Background = GlassControls.Fill(session.Contrast, session.Model.GlassTransparency);
            Opened += delegate { Attach((Border)Template.FindName("MenuSurface", this)); };
            AddHandler(MenuItem.SubmenuOpenedEvent, new RoutedEventHandler(delegate(object sender, RoutedEventArgs e) {
                var item = e.OriginalSource as MenuItem;
                if (item != null) Dispatcher.BeginInvoke(DispatcherPriority.Loaded, new Action(delegate {
                    if (item.IsSubmenuOpen) Attach((Border)item.Template.FindName("SubmenuSurface", item));
                }));
            }));
            Closed += delegate { foreach (var surface in surfaces.Values) surface.Dispose(); surfaces.Clear(); };
        }
        private void Attach(Border border)
        {
            if (surfaces.ContainsKey(border)) return;
            border.Background = GlassControls.Fill(session.Contrast, session.Model.GlassTransparency);
            var source = (HwndSource)PresentationSource.FromVisual(border);
            var glass = new NativeGlass(border, source); glass.SetBlur(session.Model.GlassBlur); surfaces.Add(border, glass);
            border.Unloaded += Release;
        }
        private void Release(object sender, RoutedEventArgs e)
        {
            var border = (Border)sender; border.Unloaded -= Release;
            NativeGlass glass;
            if (surfaces.TryGetValue(border, out glass)) { surfaces.Remove(border); glass.Dispose(); }
        }
    }
}
